import { ContentWorkspace } from '@/features/content/ContentWorkspace';

export default function SiteSettingsWorkspacePage() {
  return (
    <ContentWorkspace
      pageTitle="站点设置"
      pageDescription="站点名称、logo、首页 Hero 和页脚文案都从 site_settings 读取。要让前台生效，请确保对应条目处于已发布状态。"
      fixedTypeApiId="site_settings"
      createLabel="初始化站点设置"
      showTypeSelector={false}
      slugSearchPlaceholder="搜索站点设置 slug"
    />
  );
}