import { type ComponentType, useState } from "react";
import { Segmented } from "antd";
import { useTranslation } from "react-i18next";
import { LocalBackupSettings } from "./LocalBackupSettings";
import { WebDavBackupSettings } from "./WebDavBackupSettings";

type BackupTargetKind = "local" | "webdav";

interface BackupTargetDefinition {
  labelKey: "settings.backupCenter.local" | "settings.backupCenter.webdav";
  Component: ComponentType;
}

const BACKUP_TARGETS: Record<BackupTargetKind, BackupTargetDefinition> = {
  local: {
    labelKey: "settings.backupCenter.local",
    Component: LocalBackupSettings,
  },
  webdav: {
    labelKey: "settings.backupCenter.webdav",
    Component: WebDavBackupSettings,
  },
};

export function BackupCenter() {
  const { t } = useTranslation();
  const [activeTarget, setActiveTarget] = useState<BackupTargetKind>("local");
  const ActiveTarget = BACKUP_TARGETS[activeTarget].Component;

  return (
    <div className="p-6 pb-12">
      <Segmented
        value={activeTarget}
        options={Object.entries(BACKUP_TARGETS).map(([value, target]) => ({
          value,
          label: t(target.labelKey),
        }))}
        onChange={(value) => setActiveTarget(value as BackupTargetKind)}
        style={{ marginBottom: 16 }}
      />
      <ActiveTarget />
    </div>
  );
}
