import { Button, Card, Switch, Tag, Typography } from "antd";
import { FolderOpen, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Skill } from "@/types/domain";
import { SkillTargetIcon, skillTargetLabelKey } from "./SkillTargetIcon";

const { Paragraph, Text } = Typography;

interface Props {
  skill: Skill;
  busy: boolean;
  onDetail: (skill: Skill) => void;
  onToggle: (skill: Skill, enabled: boolean) => void;
  onOpen: (skill: Skill) => void;
  onUninstall: (skill: Skill) => void;
}

export function SkillCard({ skill, busy, onDetail, onToggle, onOpen, onUninstall }: Props) {
  const { t } = useTranslation();

  return (
    <Card size="small" className="skill-card-hover" styles={{ body: { padding: "12px 16px" } }}>
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="mb-1 flex items-center gap-2">
            <button
              type="button"
              className="skill-card-title min-w-0 cursor-pointer truncate border-0 bg-transparent p-0 text-left font-semibold"
              onClick={() => onDetail(skill)}
            >
              {skill.name}
            </button>
            <Tag className="!m-0 shrink-0">
              <span className="inline-flex items-center gap-1">
                <SkillTargetIcon target={skill.target} />
                {t(skillTargetLabelKey(skill.target))}
              </span>
            </Tag>
            {skill.version && <Text type="secondary" className="shrink-0 text-xs">v{skill.version}</Text>}
          </div>
          <Paragraph type="secondary" ellipsis={{ rows: 2 }} className="!mb-0 text-[13px]">
            {skill.description || t("skills.noDescription")}
          </Paragraph>
          {skill.author && <Text type="secondary" className="text-xs">{skill.author}</Text>}
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <Switch
            size="small"
            checked={skill.enabled}
            loading={busy}
            aria-label={t("skills.toggle", {
              name: skill.name,
              target: t(skillTargetLabelKey(skill.target)),
            })}
            onChange={(enabled) => onToggle(skill, enabled)}
          />
          <Button
            type="text"
            size="small"
            aria-label={t("skills.openDir")}
            icon={<FolderOpen size={14} />}
            onClick={() => onOpen(skill)}
          />
          <Button
            type="text"
            size="small"
            danger
            aria-label={t("skills.uninstall")}
            icon={<Trash2 size={14} />}
            onClick={() => onUninstall(skill)}
          />
        </div>
      </div>
    </Card>
  );
}
