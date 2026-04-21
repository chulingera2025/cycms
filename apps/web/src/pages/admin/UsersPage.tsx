import { useState } from 'react';
import { Button, Popconfirm, Space, Switch, Table, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { Plus } from 'lucide-react';
import { UserForm } from '@/features/users/UserForm';
import {
  useCreateUser,
  useDeleteUser,
  useUpdateUser,
  useUsers,
} from '@/features/users/hooks';
import { toast } from '@/lib/toast';
import type { CreateUserInput, UpdateUserInput, User } from '@/types';

export default function UsersPage() {
  const { data: users = [], isLoading } = useUsers();
  const create = useCreateUser();
  const update = useUpdateUser();
  const del = useDeleteUser();
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<User | null>(null);

  function openCreate() {
    setEditing(null);
    setModalOpen(true);
  }

  function openEdit(u: User) {
    setEditing(u);
    setModalOpen(true);
  }

  async function handleSubmit(values: CreateUserInput | UpdateUserInput) {
    if (editing) {
      await update.mutateAsync({ id: editing.id, input: values as UpdateUserInput });
      toast.success('用户已更新');
    } else {
      await create.mutateAsync(values as CreateUserInput);
      toast.success('用户已创建');
    }
    setModalOpen(false);
  }

  async function handleToggleActive(u: User) {
    await update.mutateAsync({ id: u.id, input: { is_active: !u.is_active } });
    toast.success(u.is_active ? `已禁用 ${u.username}` : `已启用 ${u.username}`);
  }

  const columns: ColumnsType<User> = [
    {
      title: '用户名',
      dataIndex: 'username',
      key: 'username',
      render: (v: string) => <span className="font-medium text-text">{v}</span>,
    },
    { title: '邮箱', dataIndex: 'email', key: 'email' },
    {
      title: '角色',
      dataIndex: 'roles',
      key: 'roles',
      render: (roles: string[]) =>
        roles.length === 0 ? (
          <span className="text-text-muted">—</span>
        ) : (
          <Space size={4} wrap>
            {roles.map((r) => (
              <Tag key={r}>{r}</Tag>
            ))}
          </Space>
        ),
    },
    {
      title: '状态',
      dataIndex: 'is_active',
      key: 'is_active',
      width: 96,
      render: (_: boolean, row) => (
        <Switch
          size="small"
          checked={row.is_active}
          loading={update.isPending && update.variables?.id === row.id}
          onChange={() => handleToggleActive(row)}
        />
      ),
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => new Date(v).toLocaleString('zh-CN'),
      responsive: ['md'],
    },
    {
      title: '操作',
      key: 'actions',
      width: 180,
      render: (_, row) => (
        <Space size="small">
          <Button size="small" onClick={() => openEdit(row)}>
            编辑
          </Button>
          <Popconfirm
            title="删除用户"
            description={`确定删除 ${row.username}？`}
            okButtonProps={{ danger: true }}
            okText="删除"
            cancelText="取消"
            onConfirm={async () => {
              await del.mutateAsync(row.id);
              toast.success(`已删除 ${row.username}`);
            }}
          >
            <Button size="small" danger>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div className="p-6">
      <div className="mb-4 flex items-center justify-between gap-4">
        <h1 className="m-0 text-xl font-semibold text-text">用户管理</h1>
        <Button type="primary" icon={<Plus size={14} />} onClick={openCreate}>
          新建用户
        </Button>
      </div>

      <Table<User>
        rowKey="id"
        columns={columns}
        dataSource={users}
        loading={isLoading}
        pagination={{ pageSize: 20, showSizeChanger: false }}
        scroll={{ x: 'max-content' }}
      />

      <UserForm
        open={modalOpen}
        initial={editing}
        onClose={() => setModalOpen(false)}
        onSubmit={handleSubmit}
        loading={create.isPending || update.isPending}
      />
    </div>
  );
}
