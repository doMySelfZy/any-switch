import { useMemo } from "react";
import { Select } from "antd";
import { useTranslation } from "react-i18next";
import type { Site } from "@/types/domain";
import { activeApiKey } from "@/lib/siteApiKey";
import { SiteAvatar } from "@/components/sites/SiteAvatar";

interface SiteOption {
  value: string;
  label: string;
  site: Site;
  disabled?: boolean;
}

interface Props {
  sites: Site[];
  value?: string;
  placeholder?: string;
  disabled?: boolean;
  onChange: (siteId: string) => void;
}

function SiteOptionLabel({ site, size = 18 }: { site: Site; size?: number }) {
  const active = activeApiKey(site);
  return (
    <span className="inline-flex min-w-0 items-center gap-2">
      <SiteAvatar siteId={site.id} name={site.name} baseUrl={site.baseUrl} size={size} />
      <span className="min-w-0 truncate">{site.name}</span>
      {active ? (
        <span className="min-w-0 truncate text-xs opacity-50">
          {active.label} · {active.keyPrefix}
        </span>
      ) : null}
    </span>
  );
}

function toLeaf(site: Site, disabled: boolean): SiteOption {
  return { value: site.id, label: site.name, site, disabled };
}

export function SiteSelect({ sites, value, placeholder, disabled, onChange }: Props) {
  const { t } = useTranslation();

  const options = useMemo(() => {
    const enabled = sites.filter((s) => s.enabled);
    const off = sites.filter((s) => !s.enabled);
    const groups = [];
    if (enabled.length > 0) {
      groups.push({
        label: <span>{t("apply.siteGroupEnabled")}</span>,
        title: t("apply.siteGroupEnabled"),
        options: enabled.map((site) => toLeaf(site, false)),
      });
    }
    if (off.length > 0) {
      groups.push({
        label: <span>{t("apply.siteGroupDisabled")}</span>,
        title: t("apply.siteGroupDisabled"),
        options: off.map((site) => toLeaf(site, true)),
      });
    }
    return groups;
  }, [sites, t]);

  return (
    <Select
      className="w-full"
      value={value}
      placeholder={placeholder}
      disabled={disabled}
      options={options}
      onChange={onChange}
      showSearch
      optionFilterProp="label"
      optionRender={(option) => {
        const data = option.data as SiteOption | { options?: SiteOption[] };
        if (!("site" in data) || !data.site) return option.label;
        return <SiteOptionLabel site={data.site} />;
      }}
      labelRender={(props) => {
        const site = sites.find((s) => s.id === props.value);
        if (!site) return props.label;
        return <SiteOptionLabel site={site} />;
      }}
    />
  );
}
