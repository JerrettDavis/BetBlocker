/*
 * betblocker_mf.c — BetBlocker Minifilter Driver (STUB)
 *
 * This file is a structural/documentation stub for the Wave-2 kernel driver.
 * It will NOT compile without the Windows Driver Kit (WDK) and should be
 * treated as a design reference until the full WDK build is configured.
 *
 * Architecture overview
 * ---------------------
 *  - The driver is a filesystem minifilter registered via FltRegisterFilter.
 *  - It attaches to all volumes and intercepts IRP_MJ_SET_INFORMATION (renames,
 *    deletes), IRP_MJ_WRITE, and IRP_MJ_CREATE for paths on the protected list.
 *  - An update-token mechanism allows the BetBlocker updater service to write
 *    to protected files by presenting a 32-byte HMAC token.
 *  - User-mode communicates via a minifilter communication port opened with
 *    FltCreateCommunicationPort / FltConnectCommunicationPort, using the
 *    IOCTL-style message codes defined in the Rust minifilter.rs module.
 *
 * Communication port name
 * -----------------------
 *  L"\\BetBlockerMFPort"
 *
 * IOCTL codes (matching Rust constants in minifilter.rs)
 * ------------------------------------------------------
 *  IOCTL_MF_GET_STATUS            0x00222040
 *  IOCTL_MF_ADD_PROTECTED_PATH    0x00222044
 *  IOCTL_MF_REMOVE_PROTECTED_PATH 0x00222048
 *  IOCTL_MF_SET_UPDATE_TOKEN      0x0022204C
 *
 * Build requirements (future)
 * ---------------------------
 *  Windows Driver Kit (WDK) 10.0.26100 or later
 *  Visual Studio 2022 with "Desktop development with C++" workload
 *  Compile with /kernel switch; link against FltMgr.lib, ntdll.lib
 *  Sign with EV code-signing certificate + Microsoft cross-sign (KMCS)
 */

/* -------------------------------------------------------------------------
 * WDK / DDK headers (not available without WDK — see build requirements)
 * ---------------------------------------------------------------------- */
#include <fltKernel.h>
#include <dontuse.h>
#include <suppress.h>

/* -------------------------------------------------------------------------
 * Driver constants
 * ---------------------------------------------------------------------- */
#define BETBLOCKER_MF_PORT_NAME   L"\\BetBlockerMFPort"
#define BETBLOCKER_MF_POOL_TAG    'KBMB'   /* "BMBK" reversed */

/* Maximum number of simultaneously connected user-mode clients */
#define BETBLOCKER_MF_MAX_CLIENTS 1

/* Maximum NT path length */
#define BETBLOCKER_MF_MAX_PATH    520       /* 260 wide chars * 2 bytes */

/* -------------------------------------------------------------------------
 * Message codes (must match Rust side in crates/bb-shim-windows/src/minifilter.rs)
 *
 * These are conveyed in the Code field of a FILTER_MESSAGE_HEADER /
 * FltSendMessage call rather than as IOCTL codes, because minifilters use
 * the filter communication port rather than DeviceIoControl.  The numeric
 * values are preserved for cross-layer consistency.
 * ---------------------------------------------------------------------- */
#define MF_MSG_GET_STATUS            0x00222040UL
#define MF_MSG_ADD_PROTECTED_PATH    0x00222044UL
#define MF_MSG_REMOVE_PROTECTED_PATH 0x00222048UL
#define MF_MSG_SET_UPDATE_TOKEN      0x0022204CUL

/* -------------------------------------------------------------------------
 * Wire-format structures (must match Rust structs in minifilter.rs)
 * ---------------------------------------------------------------------- */
#pragma pack(push, 1)

/*
 * MF_STATUS — output buffer for MF_MSG_GET_STATUS
 *
 *  Offset  Size  Field
 *  0       1     active           (UINT8: 0=inactive, 1=active)
 *  1       3     _pad             (reserved, write 0)
 *  4       4     protected_paths  (UINT32, little-endian)
 *  8       8     blocked_ops      (UINT64, little-endian)
 *  Total: 16 bytes
 */
typedef struct _MF_STATUS {
    UINT8  Active;
    UINT8  _Pad[3];
    UINT32 ProtectedPaths;
    UINT64 BlockedOperations;
} MF_STATUS, *PMF_STATUS;

/*
 * MF_PATH_MSG — input buffer for ADD/REMOVE_PROTECTED_PATH messages.
 * The PathBytes field holds a UTF-8 encoded NT path (no NUL terminator).
 * PathLen is the byte length of PathBytes.
 */
typedef struct _MF_PATH_MSG {
    UINT16 PathLen;
    CHAR   PathBytes[BETBLOCKER_MF_MAX_PATH];
} MF_PATH_MSG, *PMF_PATH_MSG;

#pragma pack(pop)

/* -------------------------------------------------------------------------
 * Protected-path list
 *
 * A simple fixed-size array is used here; a production implementation
 * should use an AVL tree or hash table for O(log n) / O(1) lookup.
 * ---------------------------------------------------------------------- */
#define MF_MAX_PROTECTED_PATHS 64

typedef struct _PROTECTED_PATH_ENTRY {
    BOOLEAN   InUse;
    UNICODE_STRING NtPath;
    WCHAR     Buffer[BETBLOCKER_MF_MAX_PATH / sizeof(WCHAR)];
} PROTECTED_PATH_ENTRY;

static PROTECTED_PATH_ENTRY g_ProtectedPaths[MF_MAX_PROTECTED_PATHS];
static FAST_MUTEX            g_PathListLock;
static volatile LONG         g_PathCount        = 0;
static volatile LONG64       g_BlockedOps        = 0;

/* 32-byte update authentication token (set via MF_MSG_SET_UPDATE_TOKEN) */
static UINT8 g_UpdateToken[32];
static BOOLEAN g_TokenSet = FALSE;

/* -------------------------------------------------------------------------
 * Filter registration / communication
 * ---------------------------------------------------------------------- */
static PFLT_FILTER           g_FilterHandle     = NULL;
static PFLT_PORT             g_ServerPort       = NULL;
static PFLT_PORT             g_ClientPort       = NULL;

/* -------------------------------------------------------------------------
 * Forward declarations
 * ---------------------------------------------------------------------- */
DRIVER_UNLOAD         BetBlockerMF_Unload;

static NTSTATUS       MF_ConnectNotify(
    PFLT_PORT, PVOID, PVOID, ULONG, PVOID *);
static VOID           MF_DisconnectNotify(PVOID);
static NTSTATUS       MF_MessageNotify(
    PVOID, PVOID, ULONG, PVOID, PULONG);

/* Minifilter pre-operation callbacks */
static FLT_PREOP_CALLBACK_STATUS
    PreCreate(PFLT_CALLBACK_DATA, PCFLT_RELATED_OBJECTS, PVOID *);
static FLT_PREOP_CALLBACK_STATUS
    PreWrite(PFLT_CALLBACK_DATA, PCFLT_RELATED_OBJECTS, PVOID *);
static FLT_PREOP_CALLBACK_STATUS
    PreSetInformation(PFLT_CALLBACK_DATA, PCFLT_RELATED_OBJECTS, PVOID *);

/* Path-matching helper */
static BOOLEAN IsPathProtected(PUNICODE_STRING FilePath);

/* -------------------------------------------------------------------------
 * Minifilter operation registration table
 *
 * We intercept CREATE (open for write), WRITE, and SET_INFORMATION
 * (delete / rename) on protected paths.
 * ---------------------------------------------------------------------- */
static const FLT_OPERATION_REGISTRATION g_Callbacks[] = {
    { IRP_MJ_CREATE,           0, PreCreate,         NULL },
    { IRP_MJ_WRITE,            0, PreWrite,          NULL },
    { IRP_MJ_SET_INFORMATION,  0, PreSetInformation, NULL },
    { IRP_MJ_OPERATION_END }
};

static const FLT_REGISTRATION g_FilterRegistration = {
    sizeof(FLT_REGISTRATION),       /* Size                    */
    FLT_REGISTRATION_VERSION,       /* Version                 */
    0,                              /* Flags                   */
    NULL,                           /* ContextRegistration     */
    g_Callbacks,                    /* OperationRegistration   */
    BetBlockerMF_Unload,            /* FilterUnloadCallback    */
    NULL,                           /* InstanceSetupCallback   */
    NULL,                           /* InstanceQueryTeardown   */
    NULL,                           /* InstanceTeardownStart   */
    NULL,                           /* InstanceTeardownComplete*/
    NULL,                           /* GenerateFileName        */
    NULL,                           /* NormalizeNameComponent  */
    NULL                            /* NormalizeContextCleanup */
};

/* -------------------------------------------------------------------------
 * DriverEntry
 *
 * Called by the kernel (or Filter Manager) when the driver is loaded.
 *
 * Steps:
 *  1. Register the minifilter with FltRegisterFilter.
 *  2. Create a communication port for user-mode clients.
 *  3. Start filtering with FltStartFiltering.
 * ---------------------------------------------------------------------- */
NTSTATUS
DriverEntry(
    _In_ PDRIVER_OBJECT  DriverObject,
    _In_ PUNICODE_STRING RegistryPath)
{
    NTSTATUS         status;
    UNICODE_STRING   portName;
    OBJECT_ATTRIBUTES portAttr;
    PSECURITY_DESCRIPTOR sd = NULL;

    UNREFERENCED_PARAMETER(RegistryPath);

    /* Initialise the path-list mutex */
    ExInitializeFastMutex(&g_PathListLock);
    RtlZeroMemory(g_ProtectedPaths, sizeof(g_ProtectedPaths));

    /* --- 1. Register minifilter ---------------------------------------- */
    status = FltRegisterFilter(DriverObject, &g_FilterRegistration, &g_FilterHandle);
    if (!NT_SUCCESS(status)) {
        return status;
    }

    /* --- 2. Create communication port ---------------------------------- */
    /*
     * The port security descriptor controls which processes can connect.
     * In production: restrict to SYSTEM SID so only the BetBlocker service
     * (running as LocalSystem) can connect.
     */
    status = FltBuildDefaultSecurityDescriptor(&sd, FLT_PORT_ALL_ACCESS);
    if (!NT_SUCCESS(status)) {
        FltUnregisterFilter(g_FilterHandle);
        return status;
    }

    RtlInitUnicodeString(&portName, BETBLOCKER_MF_PORT_NAME);
    InitializeObjectAttributes(
        &portAttr,
        &portName,
        OBJ_CASE_INSENSITIVE | OBJ_KERNEL_HANDLE,
        NULL,
        sd);

    status = FltCreateCommunicationPort(
        g_FilterHandle,
        &g_ServerPort,
        &portAttr,
        NULL,                       /* ServerPortCookie       */
        MF_ConnectNotify,
        MF_DisconnectNotify,
        MF_MessageNotify,
        BETBLOCKER_MF_MAX_CLIENTS);

    FltFreeSecurityDescriptor(sd);

    if (!NT_SUCCESS(status)) {
        FltUnregisterFilter(g_FilterHandle);
        return status;
    }

    /* --- 3. Start filtering -------------------------------------------- */
    status = FltStartFiltering(g_FilterHandle);
    if (!NT_SUCCESS(status)) {
        FltCloseCommunicationPort(g_ServerPort);
        FltUnregisterFilter(g_FilterHandle);
        return status;
    }

    return STATUS_SUCCESS;
}

/* -------------------------------------------------------------------------
 * BetBlockerMF_Unload
 *
 * Called by Filter Manager before unloading.  Reverse of DriverEntry.
 * ---------------------------------------------------------------------- */
NTSTATUS
BetBlockerMF_Unload(_In_ FLT_FILTER_UNLOAD_FLAGS Flags)
{
    UNREFERENCED_PARAMETER(Flags);

    if (g_ServerPort != NULL) {
        FltCloseCommunicationPort(g_ServerPort);
        g_ServerPort = NULL;
    }

    if (g_FilterHandle != NULL) {
        FltUnregisterFilter(g_FilterHandle);
        g_FilterHandle = NULL;
    }

    return STATUS_SUCCESS;
}

/* -------------------------------------------------------------------------
 * Communication port callbacks
 * ---------------------------------------------------------------------- */

static NTSTATUS
MF_ConnectNotify(
    PFLT_PORT ClientPort,
    PVOID     ServerPortCookie,
    PVOID     ConnectionContext,
    ULONG     SizeOfContext,
    PVOID    *ConnectionPortCookie)
{
    UNREFERENCED_PARAMETER(ServerPortCookie);
    UNREFERENCED_PARAMETER(ConnectionContext);
    UNREFERENCED_PARAMETER(SizeOfContext);
    UNREFERENCED_PARAMETER(ConnectionPortCookie);

    g_ClientPort = ClientPort;
    return STATUS_SUCCESS;
}

static VOID
MF_DisconnectNotify(PVOID ConnectionCookie)
{
    UNREFERENCED_PARAMETER(ConnectionCookie);
    FltCloseClientPort(g_FilterHandle, &g_ClientPort);
    g_ClientPort = NULL;
}

/*
 * MF_MessageNotify — handles messages from user-mode (bb-shim-windows).
 *
 * The InputBuffer contains a ULONG message code followed by the payload.
 * OutputBuffer is used for responses (e.g. MF_STATUS for GET_STATUS).
 */
static NTSTATUS
MF_MessageNotify(
    PVOID   ConnectionCookie,
    PVOID   InputBuffer,
    ULONG   InputBufferLength,
    PVOID   OutputBuffer,
    PULONG  OutputBufferLength)
{
    ULONG   msgCode;
    NTSTATUS status = STATUS_SUCCESS;

    UNREFERENCED_PARAMETER(ConnectionCookie);

    if (InputBuffer == NULL || InputBufferLength < sizeof(ULONG)) {
        return STATUS_INVALID_PARAMETER;
    }

    msgCode = *(ULONG *)InputBuffer;

    switch (msgCode) {

    /* ------------------------------------------------------------------
     * MF_MSG_GET_STATUS
     * Output: MF_STATUS (16 bytes)
     * ------------------------------------------------------------------ */
    case MF_MSG_GET_STATUS:
        if (OutputBuffer == NULL || *OutputBufferLength < sizeof(MF_STATUS)) {
            status = STATUS_BUFFER_TOO_SMALL;
            break;
        }
        {
            PMF_STATUS pStatus = (PMF_STATUS)OutputBuffer;
            pStatus->Active           = (g_FilterHandle != NULL) ? 1 : 0;
            pStatus->_Pad[0]          = 0;
            pStatus->_Pad[1]          = 0;
            pStatus->_Pad[2]          = 0;
            pStatus->ProtectedPaths   = (UINT32)InterlockedAdd(&g_PathCount, 0);
            pStatus->BlockedOperations= (UINT64)InterlockedAdd64(&g_BlockedOps, 0);
            *OutputBufferLength       = sizeof(MF_STATUS);
        }
        break;

    /* ------------------------------------------------------------------
     * MF_MSG_ADD_PROTECTED_PATH
     * Input: ULONG msgCode + MF_PATH_MSG payload
     * ------------------------------------------------------------------ */
    case MF_MSG_ADD_PROTECTED_PATH:
        if (InputBufferLength < sizeof(ULONG) + sizeof(UINT16)) {
            status = STATUS_INVALID_PARAMETER;
            break;
        }
        {
            PMF_PATH_MSG pMsg = (PMF_PATH_MSG)((PUCHAR)InputBuffer + sizeof(ULONG));
            /*
             * TODO (Wave 2):
             *  1. Validate PathLen.
             *  2. Convert the UTF-8 path to a UNICODE_STRING using
             *     RtlUTF8ToUnicodeN (Win10 1903+ kernel API).
             *  3. Find a free slot in g_ProtectedPaths under the g_PathListLock.
             *  4. Copy the string into the slot's Buffer and set up the
             *     UNICODE_STRING descriptor.
             *  5. InterlockedIncrement(&g_PathCount).
             */
            UNREFERENCED_PARAMETER(pMsg);
            InterlockedIncrement(&g_PathCount);
        }
        break;

    /* ------------------------------------------------------------------
     * MF_MSG_REMOVE_PROTECTED_PATH
     * Input: ULONG msgCode + MF_PATH_MSG payload
     * ------------------------------------------------------------------ */
    case MF_MSG_REMOVE_PROTECTED_PATH:
        if (InputBufferLength < sizeof(ULONG) + sizeof(UINT16)) {
            status = STATUS_INVALID_PARAMETER;
            break;
        }
        {
            PMF_PATH_MSG pMsg = (PMF_PATH_MSG)((PUCHAR)InputBuffer + sizeof(ULONG));
            /*
             * TODO (Wave 2):
             *  1. Convert PathBytes to UNICODE_STRING.
             *  2. Walk g_ProtectedPaths under g_PathListLock and find a match
             *     using RtlEqualUnicodeString.
             *  3. Zero the entry and InterlockedDecrement(&g_PathCount).
             */
            UNREFERENCED_PARAMETER(pMsg);
            if (InterlockedAdd(&g_PathCount, 0) > 0) {
                InterlockedDecrement(&g_PathCount);
            }
        }
        break;

    /* ------------------------------------------------------------------
     * MF_MSG_SET_UPDATE_TOKEN
     * Input: ULONG msgCode + 32 bytes token
     * ------------------------------------------------------------------ */
    case MF_MSG_SET_UPDATE_TOKEN:
        if (InputBufferLength < sizeof(ULONG) + 32) {
            status = STATUS_INVALID_PARAMETER;
            break;
        }
        RtlCopyMemory(
            g_UpdateToken,
            (PUCHAR)InputBuffer + sizeof(ULONG),
            32);
        g_TokenSet = TRUE;
        break;

    default:
        status = STATUS_INVALID_DEVICE_REQUEST;
        break;
    }

    return status;
}

/* -------------------------------------------------------------------------
 * Path-matching helper
 *
 * Returns TRUE if FilePath is a prefix match against any entry in
 * g_ProtectedPaths.  Holds g_PathListLock during the search.
 *
 * NOTE: must not be called at IRQL > APC_LEVEL because it acquires a
 * FAST_MUTEX.
 * ---------------------------------------------------------------------- */
static BOOLEAN
IsPathProtected(PUNICODE_STRING FilePath)
{
    BOOLEAN  found = FALSE;
    ULONG    i;

    ExAcquireFastMutex(&g_PathListLock);

    for (i = 0; i < MF_MAX_PROTECTED_PATHS; i++) {
        if (!g_ProtectedPaths[i].InUse) {
            continue;
        }
        /*
         * Prefix match: the file is "under" the protected path if FilePath
         * starts with the protected path string.
         */
        if (FilePath->Length >= g_ProtectedPaths[i].NtPath.Length) {
            UNICODE_STRING prefix = {
                g_ProtectedPaths[i].NtPath.Length,
                g_ProtectedPaths[i].NtPath.MaximumLength,
                g_ProtectedPaths[i].NtPath.Buffer
            };
            /* Case-insensitive prefix comparison */
            prefix.Length = g_ProtectedPaths[i].NtPath.Length;
            if (RtlPrefixUnicodeString(&prefix, FilePath, TRUE)) {
                found = TRUE;
                break;
            }
        }
    }

    ExReleaseFastMutex(&g_PathListLock);
    return found;
}

/* -------------------------------------------------------------------------
 * Pre-operation callbacks
 *
 * Each callback returns FLT_PREOP_SUCCESS_NO_CALLBACK if the file is NOT
 * protected, or FLT_PREOP_COMPLETE with STATUS_ACCESS_DENIED if it IS
 * protected and the operation is not authorised by a valid update token.
 * ---------------------------------------------------------------------- */

static FLT_PREOP_CALLBACK_STATUS
PreCreate(
    PFLT_CALLBACK_DATA    Data,
    PCFLT_RELATED_OBJECTS FltObjects,
    PVOID                *CompletionContext)
{
    PFLT_FILE_NAME_INFORMATION nameInfo = NULL;
    FLT_PREOP_CALLBACK_STATUS  result   = FLT_PREOP_SUCCESS_NO_CALLBACK;
    ACCESS_MASK                access;

    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(CompletionContext);

    /* Only intercept opens that request write access */
    access = Data->Iopb->Parameters.Create.SecurityContext->DesiredAccess;
    if (!(access & (FILE_WRITE_DATA | FILE_APPEND_DATA | DELETE | WRITE_DAC | WRITE_OWNER))) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    if (!NT_SUCCESS(FltGetFileNameInformation(
            Data,
            FLT_FILE_NAME_NORMALIZED | FLT_FILE_NAME_QUERY_DEFAULT,
            &nameInfo))) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    FltParseFileNameInformation(nameInfo);

    if (IsPathProtected(&nameInfo->Name)) {
        /*
         * TODO (Wave 2): check caller's EProcess token against g_UpdateToken
         * (the updater service presents its token via a preliminary
         * FltSendMessage call).  Block only if token is absent or invalid.
         */
        InterlockedIncrement64(&g_BlockedOps);
        Data->IoStatus.Status      = STATUS_ACCESS_DENIED;
        Data->IoStatus.Information = 0;
        result = FLT_PREOP_COMPLETE;
    }

    FltReleaseFileNameInformation(nameInfo);
    return result;
}

static FLT_PREOP_CALLBACK_STATUS
PreWrite(
    PFLT_CALLBACK_DATA    Data,
    PCFLT_RELATED_OBJECTS FltObjects,
    PVOID                *CompletionContext)
{
    PFLT_FILE_NAME_INFORMATION nameInfo = NULL;
    FLT_PREOP_CALLBACK_STATUS  result   = FLT_PREOP_SUCCESS_NO_CALLBACK;

    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(CompletionContext);

    if (!NT_SUCCESS(FltGetFileNameInformation(
            Data,
            FLT_FILE_NAME_NORMALIZED | FLT_FILE_NAME_QUERY_DEFAULT,
            &nameInfo))) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    FltParseFileNameInformation(nameInfo);

    if (IsPathProtected(&nameInfo->Name)) {
        /*
         * TODO (Wave 2): validate update token before allowing writes.
         */
        InterlockedIncrement64(&g_BlockedOps);
        Data->IoStatus.Status      = STATUS_ACCESS_DENIED;
        Data->IoStatus.Information = 0;
        result = FLT_PREOP_COMPLETE;
    }

    FltReleaseFileNameInformation(nameInfo);
    return result;
}

static FLT_PREOP_CALLBACK_STATUS
PreSetInformation(
    PFLT_CALLBACK_DATA    Data,
    PCFLT_RELATED_OBJECTS FltObjects,
    PVOID                *CompletionContext)
{
    PFLT_FILE_NAME_INFORMATION nameInfo = NULL;
    FLT_PREOP_CALLBACK_STATUS  result   = FLT_PREOP_SUCCESS_NO_CALLBACK;
    FILE_INFORMATION_CLASS     infoClass;

    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(CompletionContext);

    /* Intercept delete and rename only */
    infoClass = Data->Iopb->Parameters.SetFileInformation.FileInformationClass;
    if (infoClass != FileDispositionInformation      &&
        infoClass != FileDispositionInformationEx    &&
        infoClass != FileRenameInformation           &&
        infoClass != FileRenameInformationEx) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    if (!NT_SUCCESS(FltGetFileNameInformation(
            Data,
            FLT_FILE_NAME_NORMALIZED | FLT_FILE_NAME_QUERY_DEFAULT,
            &nameInfo))) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    FltParseFileNameInformation(nameInfo);

    if (IsPathProtected(&nameInfo->Name)) {
        InterlockedIncrement64(&g_BlockedOps);
        Data->IoStatus.Status      = STATUS_ACCESS_DENIED;
        Data->IoStatus.Information = 0;
        result = FLT_PREOP_COMPLETE;
    }

    FltReleaseFileNameInformation(nameInfo);
    return result;
}
