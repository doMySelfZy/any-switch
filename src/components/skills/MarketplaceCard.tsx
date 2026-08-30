import { Button, Card, Dropdown, Tag, Typography } from "antd";
import { Download, Github, Star } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { MarketplaceSkill, SkillTarget } from "@/types/domain";
import { SKILL_TARGETS, SkillTargetIcon, SkillTargetLabel, skillTargetLabelKey } from "./SkillTargetIcon";

const { Paragraph, Text } = Typography;

interface Props {
  skill: MarketplaceSkill;
  source: "skills.sh" | "github";
  installing: boolean;
  installDisabled: boolean;
  onInstall: (skill: MarketplaceSkill, target: SkillTarget) => void;
  onOpenRepo: (repo: string) => void;
}

export function MarketplaceCard({
  skill,
  source,
  installing,
  installDisabled,
  onInstall,
  onOpenRepo,
}: Props) {
  const { t } = useTranslation();
  const installed = new Set(skill.installedTargets);
  const installItems = SKILL_TARGETS.map((target) => ({
    key: target,
    label: <SkillTargetLabel target={target} />,
    disabled: installed.has(target),
  }));

  return (
    <Card size="small" className="skill-card-hover" styles={{ body: { padding: "12px 16px" } }}>
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="mb-1 flex items-center gap-2">
            <Text strong className="skill-card-title truncate">{skill.name}</Text>
            <Text type="secondary" className="inline-flex shrink-0 items-center gap-1 text-xs">
              {source === "github" ? <Star size={12} /> : <Download size={12} />}
              {(source === "github" ? skill.stars : skill.installs).toLocaleString()}
            </Text>
          </div>
          <Paragraph type="secondary" ellipsis={{ rows: 2 }} className="!mb-1 text-xs">
            {skill.description || t("skills.noDescription")}
          </Paragraph>
          <Text type="secondary" className="text-xs">{skill.repo}</Text>
          {skill.installedTargets.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1">
              {skill.installedTargets.map((target) => (
                <Tag key={target} className="!m-0">
                  <span className="inline-flex items-center gap-1">
                    <SkillTargetIcon target={target} size={12} />
                    {t(skillTargetLabelKey(target))}
                  </span>
                </Tag>
              ))}
            </div>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <Button
            type="text"
            size="small"
            icon={<Github size={14} />}
            onClick={() => onOpenRepo(skill.repo)}
          >
            GitHub
          </Button>
          <Dropdown
            trigger={["click"]}
            disabled={installDisabled || installed.size === SKILL_TARGETS.length}
            menu={{
              items: installItems,
              onClick: ({ key }) => onInstall(skill, key as SkillTarget),
            }}
          >
            <Button
              type="primary"
              size="small"
              disabled={installDisabled && !installing}
              loading={installing}
              icon={<Download size={14} />}
            >
              {installed.size === SKILL_TARGETS.length ? t("skills.installed") : t("skills.install")}
            </Button>
          </Dropdown>
        </div>
      </div>
    </Card>
  );
}
