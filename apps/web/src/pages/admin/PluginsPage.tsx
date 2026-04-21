import { useAsync } from '@/hooks/useAsync';
import { pluginsApi } from '@/lib/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';
import type { Plugin } from '@/types';

export default function PluginsPage() {
  const { data: plugins, loading, error, refetch } = useAsync(
    () => pluginsApi.list(),
    [],
  );

  if (loading) return <LoadingSpinner />;
  if (error) return <div className="page-error">加载失败: {error.message}</div>;

  return (
    <div className="page">
      <h1>插件管理</h1>
      <table className="data-table">
        <thead>
          <tr>
            <th>名称</th>
            <th>版本</th>
            <th>运行时</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {plugins?.map((p) => (
            <PluginRow key={p.name} plugin={p} onAction={refetch} />
          ))}
          {plugins?.length === 0 && (
            <tr>
              <td colSpan={5} style={{ textAlign: 'center' }}>
                暂无插件。将插件放入 plugins/ 目录后刷新。
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

function PluginRow({ plugin, onAction }: { plugin: Plugin; onAction: () => void }) {
  async function handleAction(action: 'install' | 'enable' | 'disable' | 'uninstall') {
    switch (action) {
      case 'install':
        await pluginsApi.install(plugin.name);
        break;
      case 'enable':
        await pluginsApi.enable(plugin.name);
        break;
      case 'disable':
        await pluginsApi.disable(plugin.name);
        break;
      case 'uninstall':
        if (!confirm(`确定卸载 ${plugin.name}？`)) return;
        await pluginsApi.uninstall(plugin.name);
        break;
    }
    onAction();
  }

  return (
    <tr>
      <td>{plugin.name}</td>
      <td>{plugin.version}</td>
      <td>{plugin.runtime}</td>
      <td>
        <span className={`status-badge status-${plugin.status}`}>{plugin.status}</span>
      </td>
      <td className="action-cell">
        {plugin.status === 'discovered' && (
          <button className="btn btn-sm" onClick={() => handleAction('install')}>安装</button>
        )}
        {plugin.status === 'installed' && (
          <>
            <button className="btn btn-sm btn-success" onClick={() => handleAction('enable')}>启用</button>
            <button className="btn btn-sm btn-danger" onClick={() => handleAction('uninstall')}>卸载</button>
          </>
        )}
        {plugin.status === 'enabled' && (
          <button className="btn btn-sm btn-warning" onClick={() => handleAction('disable')}>禁用</button>
        )}
        {plugin.status === 'disabled' && (
          <>
            <button className="btn btn-sm btn-success" onClick={() => handleAction('enable')}>启用</button>
            <button className="btn btn-sm btn-danger" onClick={() => handleAction('uninstall')}>卸载</button>
          </>
        )}
      </td>
    </tr>
  );
}
