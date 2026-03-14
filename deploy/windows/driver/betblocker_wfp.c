/*
 * betblocker_wfp.c — BetBlocker WFP Callout Driver (STUB)
 *
 * This file is a structural/documentation stub for the Wave-2 kernel driver.
 * It will NOT compile without the Windows Driver Kit (WDK) and should be
 * treated as a design reference until the full WDK build is configured.
 *
 * Architecture overview
 * ---------------------
 *  - The driver creates a device object at "\Device\BetBlockerWfp" and
 *    exposes it via the symbolic link "\DosDevices\BetBlockerWfp"
 *    (user-mode path: "\\.\BetBlockerWfp").
 *  - A WFP callout is registered at the FWPM_LAYER_ALE_AUTH_CONNECT_V4 and
 *    _V6 layers to intercept outbound connection attempts.
 *  - A second callout at FWPM_LAYER_DATAGRAM_DATA_V4/V6 handles UDP port 53
 *    to implement DNS redirection.
 *  - User-mode communicates with the driver via DeviceIoControl using the
 *    IOCTL codes defined in the Rust wfp.rs module (bb-shim-windows crate).
 *
 * IOCTL codes (matching Rust constants in wfp.rs)
 * ------------------------------------------------
 *  IOCTL_WFP_ADD_BLOCKED_DOMAIN    0x00222000
 *  IOCTL_WFP_REMOVE_BLOCKED_DOMAIN 0x00222004
 *  IOCTL_WFP_CLEAR_BLOCKLIST       0x00222008
 *  IOCTL_WFP_GET_STATS             0x0022200C
 *  IOCTL_WFP_SET_DNS_REDIRECT      0x00222010
 *
 * Build requirements (future)
 * ---------------------------
 *  Windows Driver Kit (WDK) 10.0.26100 or later
 *  Visual Studio 2022 with "Desktop development with C++" workload
 *  Compile with /kernel switch; link against fwpuclnt.lib, ntdll.lib
 *  Sign with EV code-signing certificate + Microsoft cross-sign (KMCS)
 */

/* -------------------------------------------------------------------------
 * WDK / DDK headers (not available without WDK — see build requirements)
 * ---------------------------------------------------------------------- */
#include <ntddk.h>
#include <wdf.h>
#include <fwpmk.h>   /* Filter Engine Management (kernel) */
#include <fwpsk.h>   /* Filter Engine Programming Interface (kernel) */
#include <initguid.h>

/* -------------------------------------------------------------------------
 * Device / symbolic-link names
 * ---------------------------------------------------------------------- */
#define BETBLOCKER_WFP_DEVICE_NAME   L"\\Device\\BetBlockerWfp"
#define BETBLOCKER_WFP_SYMLINK_NAME  L"\\DosDevices\\BetBlockerWfp"

/* -------------------------------------------------------------------------
 * IOCTL codes (must match Rust side in crates/bb-shim-windows/src/wfp.rs)
 *
 * CTL_CODE(FILE_DEVICE_UNKNOWN=0x22, Function, METHOD_BUFFERED=0, FILE_ANY_ACCESS=0)
 *   = (DeviceType << 16) | (Access << 14) | (Function << 2) | Method
 * ---------------------------------------------------------------------- */
#define IOCTL_WFP_ADD_BLOCKED_DOMAIN    CTL_CODE(0x22, 0x800, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_WFP_REMOVE_BLOCKED_DOMAIN CTL_CODE(0x22, 0x801, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_WFP_CLEAR_BLOCKLIST       CTL_CODE(0x22, 0x802, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_WFP_GET_STATS             CTL_CODE(0x22, 0x803, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_WFP_SET_DNS_REDIRECT      CTL_CODE(0x22, 0x804, METHOD_BUFFERED, FILE_ANY_ACCESS)

/* -------------------------------------------------------------------------
 * Wire-format structures (must match Rust structs in wfp.rs)
 *
 * Packed/aligned to match the Rust #[repr(C)] layout.
 * ---------------------------------------------------------------------- */
#pragma pack(push, 1)

/*
 * WFP_STATS — output buffer for IOCTL_WFP_GET_STATS
 *
 *  Offset  Size  Field
 *  0       8     blocked_queries  (UINT64, little-endian)
 *  8       4     active_rules     (UINT32, little-endian)
 *  12      4     _pad             (reserved, write 0)
 *  16      8     uptime_secs      (UINT64, little-endian)
 *  Total: 24 bytes
 */
typedef struct _WFP_STATS {
    UINT64 BlockedQueries;
    UINT32 ActiveRules;
    UINT32 _Pad;
    UINT64 UptimeSecs;
} WFP_STATS, *PWFP_STATS;

#pragma pack(pop)

/* -------------------------------------------------------------------------
 * Driver-global state
 * ---------------------------------------------------------------------- */
static PDEVICE_OBJECT  g_DeviceObject  = NULL;
static UNICODE_STRING  g_SymLinkName;

/* WFP engine handle — obtained from FwpmEngineOpen0 */
static HANDLE          g_WfpEngineHandle = NULL;

/* Callout IDs returned by FwpsCalloutRegister */
static UINT32          g_CalloutIdV4     = 0;
static UINT32          g_CalloutIdV6     = 0;

/* Simple statistics counters (updated by callout classify functions) */
static volatile LONG64 g_BlockedQueries  = 0;
static volatile LONG   g_ActiveRules     = 0;
static LARGE_INTEGER   g_LoadTime;       /* captured in DriverEntry */

/* DNS redirect port (host-byte order) */
static USHORT          g_DnsRedirectPort = 5354;

/* -------------------------------------------------------------------------
 * Forward declarations
 * ---------------------------------------------------------------------- */
DRIVER_UNLOAD           BetBlockerWfp_Unload;
DRIVER_DISPATCH         BetBlockerWfp_CreateClose;
DRIVER_DISPATCH         BetBlockerWfp_DeviceControl;

static NTSTATUS         RegisterWfpCallouts(void);
static VOID             UnregisterWfpCallouts(void);

/*
 * WFP classify callbacks — these are invoked by the filter engine on each
 * matching network event.
 */
static VOID NTAPI       Classify_AleConnectV4(
    const FWPS_INCOMING_VALUES0 *,
    const FWPS_INCOMING_METADATA_VALUES0 *,
    VOID *,
    const FWPS_FILTER1 *,
    UINT64,
    FWPS_CLASSIFY_OUT0 *);

static VOID NTAPI       Classify_DnsV4(
    const FWPS_INCOMING_VALUES0 *,
    const FWPS_INCOMING_METADATA_VALUES0 *,
    VOID *,
    const FWPS_FILTER1 *,
    UINT64,
    FWPS_CLASSIFY_OUT0 *);

/* -------------------------------------------------------------------------
 * DriverEntry
 *
 * Called by the kernel when the driver is loaded.  Steps:
 *  1. Create the device object and symbolic link for user-mode access.
 *  2. Set up IRP dispatch routines.
 *  3. Open the WFP filter engine session.
 *  4. Register WFP callouts at the desired layers.
 *  5. Add filter objects that reference the callouts.
 * ---------------------------------------------------------------------- */
NTSTATUS
DriverEntry(
    _In_ PDRIVER_OBJECT  DriverObject,
    _In_ PUNICODE_STRING RegistryPath)
{
    NTSTATUS       status;
    UNICODE_STRING deviceName;

    UNREFERENCED_PARAMETER(RegistryPath);

    /* Record load time for uptime calculation */
    KeQuerySystemTime(&g_LoadTime);

    /* --- 1. Create device object ---------------------------------------- */
    RtlInitUnicodeString(&deviceName, BETBLOCKER_WFP_DEVICE_NAME);

    status = IoCreateDevice(
        DriverObject,
        0,                          /* DeviceExtensionSize: none for now  */
        &deviceName,
        FILE_DEVICE_UNKNOWN,
        FILE_DEVICE_SECURE_OPEN,
        FALSE,                      /* Exclusive: no                       */
        &g_DeviceObject);

    if (!NT_SUCCESS(status)) {
        /* TODO: log via DbgPrint / WPP tracing */
        return status;
    }

    /* --- 2. Create symbolic link ---------------------------------------- */
    RtlInitUnicodeString(&g_SymLinkName, BETBLOCKER_WFP_SYMLINK_NAME);

    status = IoCreateSymbolicLink(&g_SymLinkName, &deviceName);
    if (!NT_SUCCESS(status)) {
        IoDeleteDevice(g_DeviceObject);
        return status;
    }

    /* --- 3. IRP dispatch table ------------------------------------------ */
    DriverObject->DriverUnload                          = BetBlockerWfp_Unload;
    DriverObject->MajorFunction[IRP_MJ_CREATE]          = BetBlockerWfp_CreateClose;
    DriverObject->MajorFunction[IRP_MJ_CLOSE]           = BetBlockerWfp_CreateClose;
    DriverObject->MajorFunction[IRP_MJ_DEVICE_CONTROL]  = BetBlockerWfp_DeviceControl;

    /* Indicate buffered I/O for the device */
    g_DeviceObject->Flags |= DO_BUFFERED_IO;
    g_DeviceObject->Flags &= ~DO_DEVICE_INITIALIZING;

    /* --- 4. Register WFP callouts --------------------------------------- */
    status = RegisterWfpCallouts();
    if (!NT_SUCCESS(status)) {
        IoDeleteSymbolicLink(&g_SymLinkName);
        IoDeleteDevice(g_DeviceObject);
        return status;
    }

    return STATUS_SUCCESS;
}

/* -------------------------------------------------------------------------
 * DriverUnload
 *
 * Called when the driver is about to be unloaded.  Clean up in reverse order:
 *  1. Remove WFP filters and unregister callouts.
 *  2. Close the WFP engine session.
 *  3. Delete the symbolic link and device object.
 * ---------------------------------------------------------------------- */
VOID
BetBlockerWfp_Unload(_In_ PDRIVER_OBJECT DriverObject)
{
    UNREFERENCED_PARAMETER(DriverObject);

    UnregisterWfpCallouts();

    if (g_WfpEngineHandle != NULL) {
        FwpmEngineClose0(g_WfpEngineHandle);
        g_WfpEngineHandle = NULL;
    }

    IoDeleteSymbolicLink(&g_SymLinkName);

    if (g_DeviceObject != NULL) {
        IoDeleteDevice(g_DeviceObject);
        g_DeviceObject = NULL;
    }
}

/* -------------------------------------------------------------------------
 * IRP_MJ_CREATE / IRP_MJ_CLOSE handler
 *
 * User-mode opens/closes the device handle.  No state per handle needed yet.
 * ---------------------------------------------------------------------- */
NTSTATUS
BetBlockerWfp_CreateClose(
    _In_ PDEVICE_OBJECT DeviceObject,
    _In_ PIRP           Irp)
{
    UNREFERENCED_PARAMETER(DeviceObject);

    Irp->IoStatus.Status      = STATUS_SUCCESS;
    Irp->IoStatus.Information = 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return STATUS_SUCCESS;
}

/* -------------------------------------------------------------------------
 * IRP_MJ_DEVICE_CONTROL handler
 *
 * Dispatches DeviceIoControl requests from user-mode (bb-shim-windows).
 *
 * All IOCTL codes use METHOD_BUFFERED, so the system copies the user-mode
 * input into Irp->AssociatedIrp.SystemBuffer before calling us, and copies
 * the output back when we complete the IRP.
 * ---------------------------------------------------------------------- */
NTSTATUS
BetBlockerWfp_DeviceControl(
    _In_ PDEVICE_OBJECT DeviceObject,
    _In_ PIRP           Irp)
{
    PIO_STACK_LOCATION  stack;
    NTSTATUS            status     = STATUS_SUCCESS;
    ULONG               bytesOut   = 0;
    PVOID               buffer;
    ULONG               inLen;
    ULONG               outLen;

    UNREFERENCED_PARAMETER(DeviceObject);

    stack  = IoGetCurrentIrpStackLocation(Irp);
    buffer = Irp->AssociatedIrp.SystemBuffer;
    inLen  = stack->Parameters.DeviceIoControl.InputBufferLength;
    outLen = stack->Parameters.DeviceIoControl.OutputBufferLength;

    switch (stack->Parameters.DeviceIoControl.IoControlCode) {

    /* ------------------------------------------------------------------
     * IOCTL_WFP_ADD_BLOCKED_DOMAIN
     * Input:  UTF-8 domain string (no NUL terminator)
     * Output: none
     * ------------------------------------------------------------------ */
    case IOCTL_WFP_ADD_BLOCKED_DOMAIN:
        if (inLen == 0 || inLen > 253) {
            status = STATUS_INVALID_PARAMETER;
            break;
        }
        /*
         * TODO (Wave 2): parse the domain from `buffer`, allocate an entry
         * in the kernel block-list (e.g. a hash table keyed on the domain
         * label sequence), and add a corresponding WFP filter via
         * FwpmFilterAdd0 that references g_CalloutIdV4/V6.
         */
        InterlockedIncrement(&g_ActiveRules);
        status = STATUS_SUCCESS;
        break;

    /* ------------------------------------------------------------------
     * IOCTL_WFP_REMOVE_BLOCKED_DOMAIN
     * Input:  UTF-8 domain string (no NUL terminator)
     * Output: none
     * ------------------------------------------------------------------ */
    case IOCTL_WFP_REMOVE_BLOCKED_DOMAIN:
        if (inLen == 0 || inLen > 253) {
            status = STATUS_INVALID_PARAMETER;
            break;
        }
        /*
         * TODO (Wave 2): look up and remove the domain entry, then call
         * FwpmFilterDeleteById0 to remove the associated WFP filter.
         */
        InterlockedDecrement(&g_ActiveRules);
        status = STATUS_SUCCESS;
        break;

    /* ------------------------------------------------------------------
     * IOCTL_WFP_CLEAR_BLOCKLIST
     * Input:  none
     * Output: none
     * ------------------------------------------------------------------ */
    case IOCTL_WFP_CLEAR_BLOCKLIST:
        /*
         * TODO (Wave 2): enumerate and remove all BetBlocker-owned WFP
         * filters via FwpmFilterDeleteById0 / FwpmFilterDestroyEnumHandle0.
         */
        InterlockedExchange(&g_ActiveRules, 0);
        status = STATUS_SUCCESS;
        break;

    /* ------------------------------------------------------------------
     * IOCTL_WFP_GET_STATS
     * Input:  none
     * Output: WFP_STATS structure (24 bytes, little-endian)
     * ------------------------------------------------------------------ */
    case IOCTL_WFP_GET_STATS:
        if (outLen < sizeof(WFP_STATS)) {
            status = STATUS_BUFFER_TOO_SMALL;
            break;
        }
        {
            PWFP_STATS   pStats = (PWFP_STATS)buffer;
            LARGE_INTEGER now, freq, elapsed;

            KeQuerySystemTime(&now);
            /* SystemTime is in 100-ns intervals; convert to seconds */
            elapsed.QuadPart = (now.QuadPart - g_LoadTime.QuadPart)
                                / 10000000LL;

            pStats->BlockedQueries = (UINT64)InterlockedAdd64(&g_BlockedQueries, 0);
            pStats->ActiveRules    = (UINT32)InterlockedAdd(&g_ActiveRules, 0);
            pStats->_Pad           = 0;
            pStats->UptimeSecs     = (UINT64)elapsed.QuadPart;

            bytesOut = sizeof(WFP_STATS);
            status   = STATUS_SUCCESS;
        }
        break;

    /* ------------------------------------------------------------------
     * IOCTL_WFP_SET_DNS_REDIRECT
     * Input:  UINT16 port (little-endian)
     * Output: none
     * ------------------------------------------------------------------ */
    case IOCTL_WFP_SET_DNS_REDIRECT:
        if (inLen < sizeof(USHORT)) {
            status = STATUS_INVALID_PARAMETER;
            break;
        }
        g_DnsRedirectPort = *(USHORT *)buffer;
        /*
         * TODO (Wave 2): update the WFP callout redirect action so that
         * intercepted DNS UDP packets are forwarded to 127.0.0.1:port.
         */
        status = STATUS_SUCCESS;
        break;

    default:
        status = STATUS_INVALID_DEVICE_REQUEST;
        break;
    }

    Irp->IoStatus.Status      = status;
    Irp->IoStatus.Information = bytesOut;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return status;
}

/* -------------------------------------------------------------------------
 * RegisterWfpCallouts
 *
 * Opens the WFP engine and registers the BetBlocker callout objects.
 *
 * Production implementation steps:
 *  1. FwpmEngineOpen0 — obtain a session handle.
 *  2. FwpmTransactionBegin0.
 *  3. FwpsCalloutRegister1 — register the classify/notifyFn/flowDeleteFn.
 *  4. FwpmCalloutAdd0 — persist the callout descriptor in the BFE database.
 *  5. FwpmSubLayerAdd0 — add a BetBlocker sub-layer (priority 0x1000).
 *  6. FwpmFilterAdd0   — add a catch-all filter referencing the callout.
 *  7. FwpmTransactionCommit0.
 * ---------------------------------------------------------------------- */
static NTSTATUS
RegisterWfpCallouts(void)
{
    NTSTATUS status;

    /*
     * TODO (Wave 2): implement the full WFP callout registration sequence
     * described above.  For now this is a no-op stub that returns success
     * so that DriverEntry can complete.
     */

    UNREFERENCED_PARAMETER(g_CalloutIdV4);
    UNREFERENCED_PARAMETER(g_CalloutIdV6);
    UNREFERENCED_PARAMETER(g_WfpEngineHandle);

    return STATUS_SUCCESS;
}

/* -------------------------------------------------------------------------
 * UnregisterWfpCallouts
 *
 * Removes all BetBlocker WFP objects from the filter engine.
 *
 * Production implementation steps (reverse of RegisterWfpCallouts):
 *  1. FwpmTransactionBegin0.
 *  2. FwpmFilterDeleteById0 for each filter.
 *  3. FwpmCalloutDeleteById0 for each callout.
 *  4. FwpmSubLayerDeleteByKey0.
 *  5. FwpmTransactionCommit0.
 *  6. FwpsCalloutUnregisterById0.
 * ---------------------------------------------------------------------- */
static VOID
UnregisterWfpCallouts(void)
{
    /* TODO (Wave 2): implement full teardown */
}

/* -------------------------------------------------------------------------
 * Classify_AleConnectV4
 *
 * WFP classify callback at FWPM_LAYER_ALE_AUTH_CONNECT_V4.
 *
 * Called for each new outbound TCP/UDP connection attempt.  If the remote IP
 * resolves to a blocked domain the action is set to FWP_ACTION_BLOCK.
 *
 * NOTE: DNS name resolution is not available inside the classify callback.
 * The production implementation will maintain a kernel-side IP→domain map
 * populated via IOCTL_WFP_ADD_BLOCKED_DOMAIN; the classify function looks
 * up the destination IP in this map and blocks if found.
 * ---------------------------------------------------------------------- */
static VOID NTAPI
Classify_AleConnectV4(
    const FWPS_INCOMING_VALUES0         *inFixedValues,
    const FWPS_INCOMING_METADATA_VALUES0 *inMetaValues,
    VOID                                *layerData,
    const FWPS_FILTER1                  *filter,
    UINT64                               flowContext,
    FWPS_CLASSIFY_OUT0                  *classifyOut)
{
    UNREFERENCED_PARAMETER(inFixedValues);
    UNREFERENCED_PARAMETER(inMetaValues);
    UNREFERENCED_PARAMETER(layerData);
    UNREFERENCED_PARAMETER(filter);
    UNREFERENCED_PARAMETER(flowContext);

    /*
     * TODO (Wave 2):
     *   1. Extract FWPS_FIELD_ALE_AUTH_CONNECT_V4_IP_REMOTE_ADDRESS.
     *   2. Look up in the in-kernel IP block set.
     *   3. If found: set classifyOut->actionType = FWP_ACTION_BLOCK, increment
     *      g_BlockedQueries, clear FWPS_RIGHT_ACTION_WRITE.
     *   4. Otherwise: set FWP_ACTION_CONTINUE.
     */
    classifyOut->actionType = FWP_ACTION_CONTINUE;
}

/* -------------------------------------------------------------------------
 * Classify_DnsV4
 *
 * WFP classify callback at FWPM_LAYER_DATAGRAM_DATA_V4 for port 53.
 *
 * Intercepts outbound DNS UDP queries and redirects them to the local
 * BetBlocker DNS sinkhole running on g_DnsRedirectPort.
 * ---------------------------------------------------------------------- */
static VOID NTAPI
Classify_DnsV4(
    const FWPS_INCOMING_VALUES0         *inFixedValues,
    const FWPS_INCOMING_METADATA_VALUES0 *inMetaValues,
    VOID                                *layerData,
    const FWPS_FILTER1                  *filter,
    UINT64                               flowContext,
    FWPS_CLASSIFY_OUT0                  *classifyOut)
{
    UNREFERENCED_PARAMETER(inFixedValues);
    UNREFERENCED_PARAMETER(inMetaValues);
    UNREFERENCED_PARAMETER(layerData);
    UNREFERENCED_PARAMETER(filter);
    UNREFERENCED_PARAMETER(flowContext);

    /*
     * TODO (Wave 2): use FwpsRedirectHandleCreate0 / FwpsQueryConnectionRedirectState0
     * to redirect DNS traffic to 127.0.0.1:g_DnsRedirectPort.
     */
    classifyOut->actionType = FWP_ACTION_CONTINUE;
}
