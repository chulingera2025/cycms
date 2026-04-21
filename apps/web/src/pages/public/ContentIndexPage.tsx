import { Card, Empty, Typography } from 'antd';
import { Link } from 'react-router-dom';
import { usePublicContentTypes } from '@/features/public/hooks';
import { PageSkeleton } from '@/components/shared/PageSkeleton';

export default function ContentIndexPage() {
  const { data: types = [], isLoading } = usePublicContentTypes();

  if (isLoading) return <PageSkeleton variant="list" />;

  return (
    <div>
      <Typography.Title level={2} style={{ marginTop: 0 }}>
        全部内容
      </Typography.Title>
      {types.length === 0 ? (
        <Empty description="暂无内容类型" />
      ) : (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {types.map((ct) => (
            <Link
              key={ct.id}
              to={`/content/${ct.api_id}`}
              className="block no-underline"
            >
              <Card hoverable>
                <Typography.Title level={4} style={{ marginTop: 0 }}>
                  {ct.name}
                </Typography.Title>
                {ct.description && (
                  <Typography.Paragraph
                    type="secondary"
                    style={{ marginBottom: 0 }}
                    ellipsis={{ rows: 2 }}
                  >
                    {ct.description}
                  </Typography.Paragraph>
                )}
              </Card>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
