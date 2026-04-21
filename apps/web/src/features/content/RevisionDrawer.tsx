import { Button, Drawer, Popconfirm, Table, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useRevisions, useRollbackRevision } from './hooks';
import { toast } from '@/lib/toast';
import type { Revision } from '@/types';

interface Props {
  open: boolean;
  typeApiId: string;
  entryId: string | undefined;
  onClose: () => void;
}

export function RevisionDrawer({ open, typeApiId, entryId, onClose }: Props) {
  const { data, isLoading } = useRevisions(typeApiId, entryId);
  const rollback = useRollbackRevision(typeApiId, entryId ?? '');

  const columns: ColumnsType<Revision> = [
    {
      title: '版本',
      dataIndex: 'version',
      key: 'version',
      width: 96,
      render: (v: number) => <Tag color="blue">v{v}</Tag>,
    },
    {
      title: '操作人',
      dataIndex: 'actor_id',
      key: 'actor_id',
      render: (v: string) => <code className="font-mono text-xs">{v.slice(0, 8)}</code>,
    },
    {
      title: '时间',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => new Date(v).toLocaleString('zh-CN'),
    },
    {
      title: '操作',
      key: 'actions',
      width: 120,
      render: (_: unknown, row) => (
        <Popconfirm
          title="回滚版本"
          description={`将内容回滚到 v${row.version}？会创建一个新版本。`}
          okText="回滚"
          cancelText="取消"
          onConfirm={async () => {
            await rollback.mutateAsync(row.version);
            toast.success(`已回滚到 v${row.version}`);
          }}
        >
          <Button size="small">回滚</Button>
        </Popconfirm>
      ),
    },
  ];

  return (
    <Drawer open={open} title="版本历史" width={720} onClose={onClose} destroyOnClose>
      <Table<Revision>
        rowKey="id"
        columns={columns}
        dataSource={data?.data ?? []}
        loading={isLoading}
        pagination={false}
        size="middle"
      />
    </Drawer>
  );
}
