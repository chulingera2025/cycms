import { useSearchParams } from 'react-router-dom';
import { ContentWorkspace } from '@/features/content/ContentWorkspace';

export default function ContentPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const routeType = searchParams.get('type') ?? undefined;

  return (
    <ContentWorkspace
      pageTitle="内容管理"
      selectedTypeApiId={routeType}
      onSelectedTypeChange={(typeApiId) => {
        const next = new URLSearchParams(searchParams);
        next.set('type', typeApiId);
        setSearchParams(next);
      }}
      createLabel="新建内容"
      slugSearchPlaceholder="搜索 slug"
    />
  );
}
