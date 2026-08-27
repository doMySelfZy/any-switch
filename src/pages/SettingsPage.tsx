import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Button, Divider, Input, InputNumber, Select, Switch, theme, App } from "antd";
import { CircleDot, ExternalLink, Github, RefreshCw, Tag } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useSettingsStore, useUIStore } from "@/stores";
import type { SettingsSection } from "@/stores/uiStore";
import { getAppVersion, PACKAGE_VERSION } from "@/lib/appVersion";
import {
  GITHUB_ISSUES_URL,
  GITHUB_RELEASES_URL,
  GITHUB_REPO_URL,
} from "@/lib/constants";
import { invoke, isAppError } from "@/lib/invoke";
import { openExternalUrl } from "@/lib/openUrl";
import type { AppPaths, AppSettings, ProxyMode, ProxyProtocol } from "@/types/domain";
import { SettingsSidebar } from "@/components/settings/SettingsSidebar";
import { SettingsGroup } from "@/components/settings/SettingsGroup";
import { BackupCenter } from "@/components/settings/BackupCenter";
import { useUpdateCheckBusy, useUpdateChecker } from "@/hooks/useUpdateChecker";
import appIconUrl from "../../assets/brand/app-icon-1024.png?url";

const rowStyle: React.CSSProperties = { padding: "4px 0" };

function GeneralSection() {
  const { t, i18n } = useTranslation();
  const { token } = theme.useToken();
  const { message } = App.useApp();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);

  const patch = async (partial: Partial<AppSettings>) => {
    try {
      await saveSettings(partial);
      if (partial.language) await i18n.changeLanguage(partial.language);
      if (typeof partial.alwaysOnTop === "boolean") {
        await invoke("set_always_on_top", { enabled: partial.alwaysOnTop }).catch(() => null);
      }
    } catch (e) {
      if (isAppError(e) && e.code === "autostart_failed") {
        message.error(t("settings.autoStartFailed"));
        return;
      }
      message.error(isAppError(e) ? e.message : t("settings.saveFailed"));
    }
  };

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title={t("settings.groupAppearance")}>
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <span>{t("settings.language")}</span>
          <Select
            size="small"
            style={{ minWidth: 140 }}
            value={settings.language}
            onChange={(language) => void patch({ language })}
            options={[
              { value: "zh-CN", label: "简体中文" },
              { value: "en-US", label: "English" },
            ]}
          />
        </div>
        <Divider style={{ margin: "8px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <span>{t("settings.theme")}</span>
          <Select
            size="small"
            style={{ minWidth: 140 }}
            value={settings.themeMode}
            onChange={(themeMode) => void patch({ themeMode })}
            options={[
              { value: "system", label: t("settings.themeSystem") },
              { value: "light", label: t("settings.themeLight") },
              { value: "dark", label: t("settings.themeDark") },
            ]}
          />
        </div>
      </SettingsGroup>

      <SettingsGroup title={t("settings.groupWindow")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.alwaysOnTop")}</span>
          <Switch checked={settings.alwaysOnTop} onChange={(alwaysOnTop) => void patch({ alwaysOnTop })} />
        </div>
        <Divider style={{ margin: "8px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.autoStart")}</span>
          <Switch checked={settings.autoStart} onChange={(autoStart) => void patch({ autoStart })} />
        </div>
        <Divider style={{ margin: "8px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <div>{t("settings.closeToTray")}</div>
            <div className="text-xs" style={{ color: token.colorTextSecondary }}>
              {t("settings.closeToTrayHint")}
            </div>
          </div>
          <Switch
            checked={settings.closeToTray}
            onChange={(closeToTray) => void patch({ closeToTray })}
          />
        </div>
        <Divider style={{ margin: "8px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <div>{t("settings.startInTray")}</div>
            <div className="text-xs" style={{ color: token.colorTextSecondary }}>
              {settings.closeToTray ? t("settings.startInTrayHint") : t("settings.startInTrayDisabled")}
            </div>
          </div>
          <Switch
            checked={settings.startInTray}
            disabled={!settings.closeToTray}
            onChange={(startInTray) => void patch({ startInTray })}
          />
        </div>
      </SettingsGroup>

      <SettingsGroup title={t("settings.groupApply")}>
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <div>
            <div>{t("settings.forceExclusiveAuth")}</div>
            <div style={{ color: "var(--color-text-secondary)", fontSize: 12 }}>
              {t("settings.forceExclusiveAuthHint")}
            </div>
          </div>
          <Switch
            checked={settings.forceExclusiveClaudeAuthKey}
            onChange={(forceExclusiveClaudeAuthKey) => void patch({ forceExclusiveClaudeAuthKey })}
          />
        </div>
        <Divider style={{ margin: "8px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <span>{t("settings.codexEnvMode")}</span>
          <Select
            size="small"
            style={{ minWidth: 180 }}
            value={settings.codexEnvInjectMode}
            onChange={(codexEnvInjectMode) => void patch({ codexEnvInjectMode })}
            options={[
              { value: "auto", label: t("settings.codexEnvAuto") },
              { value: "shell_rc", label: t("settings.codexEnvShell") },
              { value: "user_env", label: t("settings.codexEnvUser") },
              { value: "file_only", label: t("settings.codexEnvFile") },
            ]}
          />
        </div>
      </SettingsGroup>
    </div>
  );
}

function NetworkSection() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [host, setHost] = useState(settings.proxyHost ?? "");
  const [port, setPort] = useState<number | null>(settings.proxyPort ?? null);

  useEffect(() => {
    setHost(settings.proxyHost ?? "");
    setPort(settings.proxyPort ?? null);
  }, [settings.proxyHost, settings.proxyPort]);

  const patch = async (partial: Partial<AppSettings>) => {
    await saveSettings(partial);
  };

  const saveHost = () => {
    const next = host.trim();
    if (next === (settings.proxyHost ?? "")) return;
    if (!next) {
      void patch({ proxyHost: null });
      return;
    }
    void patch({ proxyHost: next });
  };

  const savePort = (value: number | null) => {
    setPort(value);
    if (value == null || !Number.isFinite(value)) return;
    const next = Math.min(65535, Math.max(1, Math.round(value)));
    if (next === settings.proxyPort) return;
    void patch({ proxyPort: next });
  };

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title={t("settings.groupProxy")}>
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <div>{t("settings.proxyMode")}</div>
            {settings.proxyMode === "system" && (
              <div className="text-xs" style={{ color: token.colorTextSecondary }}>
                {t("settings.proxySystemHint")}
              </div>
            )}
          </div>
          <Select
            size="small"
            style={{ minWidth: 160 }}
            value={settings.proxyMode}
            onChange={(proxyMode: ProxyMode) => void patch({ proxyMode })}
            options={[
              { value: "system", label: t("settings.proxySystem") },
              { value: "none", label: t("settings.proxyNone") },
              { value: "custom", label: t("settings.proxyCustom") },
            ]}
          />
        </div>
        {settings.proxyMode === "custom" && (
          <>
            <Divider style={{ margin: "8px 0" }} />
            <div style={rowStyle} className="flex items-center justify-between gap-4">
              <span>{t("settings.proxyProtocol")}</span>
              <Select
                size="small"
                style={{ minWidth: 160 }}
                value={settings.proxyProtocol}
                onChange={(proxyProtocol: ProxyProtocol) => void patch({ proxyProtocol })}
                options={[
                  { value: "http", label: "HTTP" },
                  { value: "https", label: "HTTPS" },
                  { value: "socks5", label: "SOCKS5" },
                ]}
              />
            </div>
            <Divider style={{ margin: "8px 0" }} />
            <div style={rowStyle} className="flex items-center justify-between gap-4">
              <span>{t("settings.proxyHost")}</span>
              <Input
                size="small"
                style={{ width: 220 }}
                value={host}
                allowClear
                placeholder="127.0.0.1"
                onChange={(e) => setHost(e.target.value)}
                onBlur={saveHost}
                onPressEnter={saveHost}
              />
            </div>
            <Divider style={{ margin: "8px 0" }} />
            <div style={rowStyle} className="flex items-center justify-between gap-4">
              <span>{t("settings.proxyPort")}</span>
              <InputNumber
                size="small"
                min={1}
                max={65535}
                precision={0}
                style={{ width: 160 }}
                value={port}
                onChange={savePort}
              />
            </div>
          </>
        )}
      </SettingsGroup>

      <SettingsGroup title={t("settings.groupProbe")}>
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <div>{t("settings.probeTtl")}</div>
            <div className="text-xs" style={{ color: token.colorTextSecondary }}>
              {t("settings.probeTtlHint")}
            </div>
          </div>
          <InputNumber
            min={1}
            max={1440}
            precision={0}
            style={{ width: 140 }}
            value={settings.routeProbeTtlMinutes}
            addonAfter={t("settings.probeTtlUnit")}
            onChange={(v) => {
              if (v == null || !Number.isFinite(v)) return;
              void patch({ routeProbeTtlMinutes: Math.min(1440, Math.max(1, Math.round(v))) });
            }}
          />
        </div>
      </SettingsGroup>
    </div>
  );
}

function PathsSection() {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [claude, setClaude] = useState(settings.claudeHomeOverride ?? "");
  const [codex, setCodex] = useState(settings.codexHomeOverride ?? "");
  const [pi, setPi] = useState(settings.piAgentDirOverride ?? "");

  useEffect(() => {
    setClaude(settings.claudeHomeOverride ?? "");
    setCodex(settings.codexHomeOverride ?? "");
    setPi(settings.piAgentDirOverride ?? "");
  }, [settings.claudeHomeOverride, settings.codexHomeOverride, settings.piAgentDirOverride]);

  const onSave = async () => {
    await saveSettings({
      claudeHomeOverride: claude.trim() || null,
      codexHomeOverride: codex.trim() || null,
      piAgentDirOverride: pi.trim() || null,
    });
    message.success(t("settings.pathsSaved"));
  };

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title={t("settings.paths")}>
        <div className="mb-3">
          <div className="mb-1 text-sm">{t("settings.claudeHome")}</div>
          <Input
            value={claude}
            onChange={(e) => setClaude(e.target.value)}
            placeholder={t("settings.pathPlaceholder")}
          />
        </div>
        <div className="mb-3">
          <div className="mb-1 text-sm">{t("settings.piAgentDir")}</div>
          <Input
            value={pi}
            onChange={(event) => setPi(event.target.value)}
            placeholder={t("settings.piAgentDirPlaceholder")}
          />
          <div className="mt-1 text-xs opacity-50">{t("settings.piAgentDirHint")}</div>
        </div>
        <div className="mb-3">
          <div className="mb-1 text-sm">{t("settings.codexHome")}</div>
          <Input
            value={codex}
            onChange={(e) => setCodex(e.target.value)}
            placeholder={t("settings.pathPlaceholder")}
          />
        </div>
        <Button type="primary" onClick={() => void onSave()}>
          {t("settings.save")}
        </Button>
      </SettingsGroup>
    </div>
  );
}

function BackupSection() {
  return <BackupCenter />;
}

function displayUrl(url: string): string {
  return url.replace(/^https:\/\//, "");
}

function AboutLinkRow({
  icon,
  label,
  url,
}: {
  icon: React.ReactNode;
  label: string;
  url: string;
}) {
  const { token } = theme.useToken();
  const [hovered, setHovered] = useState(false);

  return (
    <button
      type="button"
      onClick={() => void openExternalUrl(url)}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      className="flex w-full items-center justify-between gap-3"
      style={{
        margin: "0 -8px",
        padding: "8px",
        border: "none",
        borderRadius: 8,
        background: hovered ? token.colorFillTertiary : "transparent",
        cursor: "pointer",
        color: token.colorText,
        textAlign: "left",
      }}
    >
      <span className="flex min-w-0 items-center gap-2.5">
        <span className="inline-flex shrink-0" style={{ color: token.colorTextSecondary }}>
          {icon}
        </span>
        <span className="min-w-0">
          <span className="block">{label}</span>
          <span className="block truncate text-xs" style={{ color: token.colorTextTertiary }}>
            {displayUrl(url)}
          </span>
        </span>
      </span>
      <ExternalLink size={14} className="shrink-0" style={{ color: token.colorTextQuaternary }} />
    </button>
  );
}

function AboutSection() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { checkForUpdate } = useUpdateChecker();
  const checking = useUpdateCheckBusy();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [paths, setPaths] = useState<AppPaths | null>(null);
  const [version, setVersion] = useState(PACKAGE_VERSION);

  useEffect(() => {
    void invoke<AppPaths>("get_app_paths")
      .then(setPaths)
      .catch(() => null);
    void getAppVersion()
      .then(setVersion)
      .catch(() => null);
  }, []);

  const handleCheckUpdate = () => {
    void checkForUpdate();
  };

  const patch = async (partial: Partial<AppSettings>) => {
    await saveSettings(partial);
  };

  return (
    <div className="p-6 pb-12">
      <SettingsGroup>
        <div className="flex flex-col items-center py-3 text-center">
          <img src={appIconUrl} alt={t("app.name")} width={88} height={88} draggable={false} />
          <div className="mt-3 text-lg font-semibold" style={{ color: token.colorText }}>
            {t("app.name")}
          </div>
          <div className="mt-1 text-sm" style={{ color: token.colorTextSecondary }}>
            {t("app.tagline")}
          </div>
          <div className="mt-2 text-xs" style={{ color: token.colorTextTertiary }}>
            {t("settings.version")} <span>{version}</span>
          </div>
        </div>
      </SettingsGroup>
      <SettingsGroup title={t("settings.groupUpdate")}>
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <span>{t("settings.checkUpdate")}</span>
          <Button
            icon={<RefreshCw size={16} />}
            loading={checking}
            disabled={checking}
            onClick={handleCheckUpdate}
          >
            {checking ? t("settings.checkingUpdate") : t("settings.checkUpdate")}
          </Button>
        </div>
        <Divider style={{ margin: "8px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <span>{t("settings.autoCheckUpdate")}</span>
          <Switch
            checked={settings.autoCheckUpdate}
            onChange={(autoCheckUpdate) => void patch({ autoCheckUpdate })}
          />
        </div>
        <Divider style={{ margin: "8px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between gap-4">
          <span>{t("settings.updateCheckInterval")}</span>
          <InputNumber
            min={1}
            max={1440}
            precision={0}
            style={{ width: 140 }}
            value={settings.updateCheckInterval}
            disabled={!settings.autoCheckUpdate}
            addonAfter={t("settings.minutes")}
            onChange={(v) => {
              if (v == null || !Number.isFinite(v)) return;
              void patch({
                updateCheckInterval: Math.min(1440, Math.max(1, Math.round(v))),
              });
            }}
          />
        </div>
      </SettingsGroup>
      <SettingsGroup title={t("settings.linksTitle")}>
        <AboutLinkRow icon={<Github size={16} />} label={t("settings.githubRepo")} url={GITHUB_REPO_URL} />
        <Divider style={{ margin: "4px 0" }} />
        <AboutLinkRow icon={<CircleDot size={16} />} label={t("settings.githubIssues")} url={GITHUB_ISSUES_URL} />
        <Divider style={{ margin: "4px 0" }} />
        <AboutLinkRow icon={<Tag size={16} />} label={t("settings.githubReleases")} url={GITHUB_RELEASES_URL} />
      </SettingsGroup>
      <SettingsGroup title={t("settings.securityTitle")}>
        <p className="m-0 text-sm" style={{ color: token.colorTextSecondary }}>
          {t("settings.securityBody")}
        </p>
      </SettingsGroup>
      {paths && (
        <SettingsGroup title={t("settings.plaintextPaths")}>
          <ul className="m-0 list-disc space-y-1 pl-5 font-mono text-xs">
            <li>{paths.appDir}</li>
            <li>{paths.codexEnvPath}</li>
            <li>~/.claude/settings.json</li>
            <li>~/.codex/config.toml</li>
            <li>~/.pi/agent/auth.json</li>
          </ul>
          <Button className="mt-3" onClick={() => void invoke("open_path", { path: paths.appDir })}>
            {t("settings.openAppDir")}
          </Button>
        </SettingsGroup>
      )}
    </div>
  );
}

const SECTION_COMPONENTS: Record<SettingsSection, React.ComponentType> = {
  general: GeneralSection,
  network: NetworkSection,
  paths: PathsSection,
  backup: BackupSection,
  about: AboutSection,
};

export function SettingsPage() {
  const { token } = theme.useToken();
  const settingsTab = useUIStore((s) => s.settingsTab);
  const setPage = useUIStore((s) => s.setPage);
  const fetchSettings = useSettingsStore((s) => s.fetchSettings);
  const ContentComponent = SECTION_COMPONENTS[settingsTab];
  const contentScrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void fetchSettings();
  }, [fetchSettings]);

  useLayoutEffect(() => {
    const el = contentScrollRef.current;
    if (!el) return;
    el.scrollTop = 0;
    el.scrollLeft = 0;
  }, [settingsTab]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        setPage("sites");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setPage]);

  return (
    <div className="flex h-full min-h-0">
      <div
        className="h-full w-56 shrink-0"
        style={{ borderRight: "1px solid var(--border-color)", backgroundColor: token.colorBgContainer }}
      >
        <SettingsSidebar />
      </div>
      <div
        ref={contentScrollRef}
        className="min-w-0 flex-1 overflow-y-auto"
        style={{ backgroundColor: token.colorBgElevated }}
      >
        <ContentComponent />
      </div>
    </div>
  );
}
