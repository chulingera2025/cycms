import { useMemo } from 'react';
import { Empty, Input, List, Select, Space, Typography } from 'antd';
import { Link, useSearchParams } from 'react-router-dom';
import {
  usePublicContentList,
  usePublicContentTypes,
} from '@/features/public/hooks';
import { PageSkeleton } from '@/components/shared/PageSkeleton';
import { formatDateTime } from '@/utils/format';

export default function SearchPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const q = searchParams.get('q') ?? '';
  const typeFromUrl = searchParams.get('type') ?? '';

  const { data: types = [] } = usePublicContentTypes();
  const typeApiId = typeFromUrl || types[0]?.api_id || '';

  const params = useMemo(
    (): Record<string, string> =>
      q ? { 'filter[slug][contains]': q, pageSize: '20' } : {},
    [q],
  );

  const { data, isLoading } = usePublicContentList(q ? typeApiId : undefined, params);

  function updateParam(key: string, value: string) {
    const next = new URLSearchParams(searchParams);
    if (value) next.set(key, value);
    else next.delete(key);
    setSearchParams(next);
  }

  return (
    <div>
      <Typography.Title level={2} style={{ marginTop: 0 }}>
        搜索
      </Typography.Title>

      <Space.Compact style={{ width: '100%', maxWidth: 640 }}>
        <Select
          value={typeApiId}
          onChange={(v) => updateParam('type', v)}
          style={{ width: 160 }}
          options={types.map((t) => ({ value: t.api_id, label: t.name }))}
          placeholder="内容类型"
        />
        <Input.Search
          placeholder="搜索 slug..."
          defaultValue={q}
          allowClear
          enterButton
          onSearch={(v) => updateParam('q', v)}
        />
      </Space.Compact>

      {isLoading && <PageSkeleton variant="list" />}

      {q && !isLoading && (
        <div className="mt-6">
          <Typography.Text type="secondary">
            共 {data?.meta.total ?? 0} 条结果
          </Typography.Text>
          {(data?.data ?? []).length === 0 ? (
            <Empty description="未找到匹配的内容" className="mt-6" />
          ) : (
            <List
              className="mt-3"
              dataSource={data?.data ?? []}
              renderItem={(entry) => (
                <List.Item>
                  <List.Item.Meta
                    title={
                      <Link
                        to={`/content/${typeApiId}/${entry.slug ?? entry.id}`}
                        className="text-text hover:text-brand"
                      >
                        {entry.slug ?? entry.id.slice(0, 8)}
                      </Link>
                    }
                    description={formatDateTime(
                      entry.published_at ?? entry.created_at,
                    )}
                  />
                </List.Item>
              )}
            />
          )}
        </div>
      )}
    </div>
  );
}
