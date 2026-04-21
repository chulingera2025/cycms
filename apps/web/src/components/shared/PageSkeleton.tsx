import { Skeleton } from 'antd';

type Variant = 'list' | 'detail' | 'dashboard';

export function PageSkeleton({ variant = 'list' }: { variant?: Variant }) {
  if (variant === 'dashboard') {
    return (
      <div className="grid grid-cols-1 gap-4 p-6 md:grid-cols-2 lg:grid-cols-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i} className="rounded-lg border border-border bg-surface p-4">
            <Skeleton active paragraph={{ rows: 2 }} />
          </div>
        ))}
      </div>
    );
  }
  if (variant === 'detail') {
    return (
      <div className="p-6">
        <Skeleton active title paragraph={{ rows: 1 }} />
        <div className="mt-6">
          <Skeleton active paragraph={{ rows: 6 }} />
        </div>
      </div>
    );
  }
  return (
    <div className="p-6">
      <div className="mb-4 flex justify-between">
        <Skeleton.Input active style={{ width: 220 }} />
        <Skeleton.Button active />
      </div>
      <Skeleton active paragraph={{ rows: 8 }} />
    </div>
  );
}
