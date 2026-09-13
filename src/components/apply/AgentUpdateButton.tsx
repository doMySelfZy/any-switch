import { Button, message } from 'antd';
import { Download } from 'lucide-react';
import { useAgentUpdateStore } from '@/stores/agentUpdateStore';
import { useTranslation } from 'react-i18next';

export function AgentUpdateButton() {
  const { t } = useTranslation();
  const hasAnyUpdate = useAgentUpdateStore((s) => s.hasAnyUpdate());
  const updateCount = useAgentUpdateStore((s) => s.updateCount());
  const updating = useAgentUpdateStore((s) => s.updating);
  const updateAll = useAgentUpdateStore((s) => s.updateAll);

  if (!hasAnyUpdate) {
    return null;
  }

  const handleUpdateAll = async () => {
    try {
      const result = await updateAll();

      if (result.failures.length === 0) {
        message.success(t('apply.agentUpdateAllSuccess', { count: result.successes.length }));
      } else {
        message.warning(
          t('apply.agentUpdatePartial', {
            success: result.successes.length,
            failed: result.failures.length,
          })
        );
      }
    } catch (error) {
      message.error(t('apply.agentUpdateFailed'));
    }
  };

  return (
    <Button
      type="primary"
      size="small"
      icon={<Download size={14} />}
      loading={updating}
      onClick={() => void handleUpdateAll()}
    >
      {t('apply.updateAllAgents', { count: updateCount })}
    </Button>
  );
}
