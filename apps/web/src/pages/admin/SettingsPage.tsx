import { useEffect, useMemo, useState } from 'react';
import { Alert, Tabs } from 'antd';
import { useAdminExtensions } from '@/features/admin-extensions';
import { useSettingSchemas } from '@/features/settings/hooks';
import {
  CustomSettingsPanel,
  RawNamespacePanel,
  SchemaNamespacePanel,
} from '@/features/settings/panels';
import { CORE_SETTINGS_NAMESPACES } from '@/features/settings/namespaces';

export default function SettingsPage() {
  const { settingsNamespaces } = useAdminExtensions();
  const { data: schemas = [], isLoading: schemasLoading } = useSettingSchemas();
  const [active, setActive] = useState('system');

  const pluginTabs = useMemo(() => {
    const schemaMap = new Map(schemas.map((schema) => [schema.plugin_name, schema]));
    const namespaces = new Set<string>([
      ...schemas.map((schema) => schema.plugin_name),
      ...settingsNamespaces.map((entry) => entry.namespace),
    ]);

    return [...namespaces]
      .sort((left, right) => left.localeCompare(right, 'zh-CN'))
      .map((namespace) => {
        const schema = schemaMap.get(namespace);
        const settingsNamespace = settingsNamespaces.find((entry) => entry.namespace === namespace) ?? null;

        return {
          key: namespace,
          label: settingsNamespace?.contribution.customPage ? `${namespace} · 插件页` : namespace,
          children: settingsNamespace?.contribution.customPage ? (
            <CustomSettingsPanel
              namespace={namespace}
              pluginName={settingsNamespace.pluginName}
              contribution={settingsNamespace.contribution}
            />
          ) : schema ? (
            <SchemaNamespacePanel namespace={namespace} schema={schema.schema} />
          ) : (
            <RawNamespacePanel namespace={namespace} />
          ),
        };
      });
  }, [schemas, settingsNamespaces]);

  const tabItems = useMemo(
    () => [
      ...CORE_SETTINGS_NAMESPACES.map((namespace) => ({
        key: namespace.key,
        label: namespace.label,
        children: <RawNamespacePanel namespace={namespace.key} />,
      })),
      ...pluginTabs,
    ],
    [pluginTabs],
  );

  useEffect(() => {
    if (!tabItems.some((item) => item.key === active) && tabItems[0]) {
      setActive(tabItems[0].key);
    }
  }, [active, tabItems]);

  return (
    <div className="p-6">
      <div className="mb-4">
        <h1 className="m-0 text-xl font-semibold text-text">系统设置</h1>
        <p className="mt-1 text-sm text-text-muted">
          核心命名空间使用宿主原生表格编辑；插件命名空间会优先消费 bootstrap registry 和 settings schema。
        </p>
      </div>
      {schemasLoading && (
        <Alert
          className="mb-4"
          type="info"
          showIcon
          message="正在同步插件 settings schema"
          description="插件命名空间和自定义设置页会在 schema 加载完成后自动并入当前页面。"
        />
      )}
      <Tabs
        activeKey={active}
        onChange={setActive}
        items={tabItems}
      />
    </div>
  );
}
