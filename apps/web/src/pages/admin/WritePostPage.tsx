import { ContentWorkspace } from '@/features/content/ContentWorkspace';

export default function WritePostPage() {
  return (
    <ContentWorkspace
      pageTitle="写文章"
      pageDescription="进入时会直接打开新文章编辑器；保存后仍可在本页继续管理草稿和已发布文章。"
      fixedTypeApiId="post"
      createLabel="新建文章"
      autoOpenCreate
      defaultStatusFilter="draft"
      showTypeSelector={false}
      slugSearchPlaceholder="搜索文章 slug"
    />
  );
}