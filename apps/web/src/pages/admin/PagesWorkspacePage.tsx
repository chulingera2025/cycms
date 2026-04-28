import { ContentWorkspace } from '@/features/content/ContentWorkspace';

export default function PagesWorkspacePage() {
  return (
    <ContentWorkspace
      pageTitle="管理页面"
      pageDescription="集中维护关于页、落地页和其他常驻页面。页面模型默认带有封面、正文和 SEO 字段。"
      fixedTypeApiId="page"
      createLabel="新建页面"
      showTypeSelector={false}
      slugSearchPlaceholder="搜索页面 slug"
    />
  );
}