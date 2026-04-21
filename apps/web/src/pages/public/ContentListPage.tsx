import { useMemo, useState } from 'react';
import { Card, Empty, Pagination, Typography } from 'antd';
import { Link, useParams } from 'react-router-dom';
import { usePublicContentList } from '@/features/public/hooks';
import { PageSkeleton } from '@/components/shared/PageSkeleton';
import { formatDateTime } from '@/utils/format';

export default function ContentListPage() {
  const { typeApiId } = useParams<{ typeApiId: string }>();
  const [page, setPage] = useState(1);
  const pageSize = 12;

  const params = useMemo(
    () => ({ page: String(page), pageSize: String(pageSize) }),
    [page],
  );

  const { data, isLoading } = usePublicContentList(typeApiId, params);

  if (!typeApiId) return <Empty description="请选择内容类型" />;
  if (isLoading) return <PageSkeleton variant="list" />;

  return (
    <div>
      <Typography.Title level={2} style={{ marginTop: 0, textTransform: 'capitalize' }}>
        {typeApiId.replace(/_/g, ' ')}
      </Typography.Title>

      {data && data.data.length === 0 ? (
        <Empty description="暂无已发布的内容" />
      ) : (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {data?.data.map((entry) => (
            <Link
              key={entry.id}
              to={`/content/${typeApiId}/${entry.slug ?? entry.id}`}
              className="block no-underline"
            >
              <Card hoverable>
                <Typography.Title level={5} style={{ marginTop: 0 }}>
                  {entry.slug ?? entry.id.slice(0, 8)}
                </Typography.Title>
                <Typography.Text type="secondary" className="text-xs">
                  {formatDateTime(entry.published_at ?? entry.created_at)}
                </Typography.Text>
              </Card>
            </Link>
          ))}
        </div>
      )}

      {data && data.meta.page_count > 1 && (
        <div className="mt-6 flex justify-center">
          <Pagination
            current={page}
            pageSize={pageSize}
            total={data.meta.total}
            onChange={setPage}
            showSizeChanger={false}
          />
        </div>
      )}
    </div>
  );
}
