'use client';

import type { ProtectionConfig, ReportingConfig } from '@/lib/api-types';
import { Badge } from '@/components/ui/badge';
import { capitalize } from '@/lib/utils';

interface EnrollmentConfigProps {
  protection: ProtectionConfig;
  reporting: ReportingConfig;
}

export function EnrollmentConfig({ protection, reporting }: EnrollmentConfigProps) {
  return (
    <div className="grid gap-6 md:grid-cols-2">
      <div>
        <h4 className="text-sm font-semibold mb-3">Protection Config</h4>
        <div className="space-y-2 text-sm">
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">DNS Blocking</span>
            <Badge variant={protection.dns_blocking ? 'default' : 'secondary'}>
              {protection.dns_blocking ? 'Enabled' : 'Disabled'}
            </Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">App Blocking</span>
            <Badge variant={protection.app_blocking ? 'default' : 'secondary'}>
              {protection.app_blocking ? 'Enabled' : 'Disabled'}
            </Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Browser Blocking</span>
            <Badge variant={protection.browser_blocking ? 'default' : 'secondary'}>
              {protection.browser_blocking ? 'Enabled' : 'Disabled'}
            </Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">VPN Detection</span>
            <Badge variant="outline">{capitalize(protection.vpn_detection)}</Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Tamper Response</span>
            <Badge variant="outline">{capitalize(protection.tamper_response)}</Badge>
          </div>
        </div>
      </div>

      <div>
        <h4 className="text-sm font-semibold mb-3">Reporting Config</h4>
        <div className="space-y-2 text-sm">
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Reporting Level</span>
            <Badge variant="outline">{capitalize(reporting.level)}</Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Block Counts</span>
            <Badge variant={reporting.blocked_attempt_counts ? 'default' : 'secondary'}>
              {reporting.blocked_attempt_counts ? 'Yes' : 'No'}
            </Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Domain Details</span>
            <Badge variant={reporting.domain_details ? 'default' : 'secondary'}>
              {reporting.domain_details ? 'Yes' : 'No'}
            </Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Tamper Alerts</span>
            <Badge variant={reporting.tamper_alerts ? 'default' : 'secondary'}>
              {reporting.tamper_alerts ? 'Yes' : 'No'}
            </Badge>
          </div>
        </div>
      </div>
    </div>
  );
}
