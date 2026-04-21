import { useState } from 'react';
import { Button, Card, Empty, Popconfirm, Space, Tag } from 'antd';
import { Plus } from 'lucide-react';
import { RoleForm } from '@/features/roles/RoleForm';
import {
  useCreateRole,
  useDeleteRole,
  useRoles,
  useUpdateRole,
} from '@/features/roles/hooks';
import type { RoleFormValues } from '@/features/roles/schema';
import { toast } from '@/lib/toast';
import type { Role } from '@/types';

export default function RolesPage() {
  const { data: roles = [], isLoading } = useRoles();
  const create = useCreateRole();
  const update = useUpdateRole();
  const del = useDeleteRole();
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<Role | null>(null);

  function openCreate() {
    setEditing(null);
    setOpen(true);
  }
  function openEdit(r: Role) {
    setEditing(r);
    setOpen(true);
  }

  async function handleSubmit(values: RoleFormValues) {
    if (editing) {
      await update.mutateAsync({ id: editing.id, input: values });
      toast.success('角色已更新');
    } else {
      await create.mutateAsync(values);
      toast.success('角色已创建');
    }
    setOpen(false);
  }

  return (
    <div className="p-6">
      <div className="mb-4 flex items-center justify-between gap-4">
        <h1 className="m-0 text-xl font-semibold text-text">角色与权限</h1>
        <Button type="primary" icon={<Plus size={14} />} onClick={openCreate}>
          新建角色
        </Button>
      </div>

      {isLoading ? null : roles.length === 0 ? (
        <Empty description="暂无角色" />
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
          {roles.map((role) => (
            <Card
              key={role.id}
              title={
                <div className="flex items-center gap-2">
                  <span>{role.name}</span>
                  {role.is_system && <Tag color="gold">系统</Tag>}
                </div>
              }
              extra={
                <Space size="small">
                  <Button size="small" onClick={() => openEdit(role)}>
                    {role.is_system ? '查看' : '编辑'}
                  </Button>
                  {!role.is_system && (
                    <Popconfirm
                      title="删除角色"
                      description={`确定删除 ${role.name}？`}
                      okButtonProps={{ danger: true }}
                      okText="删除"
                      cancelText="取消"
                      onConfirm={async () => {
                        await del.mutateAsync(role.id);
                        toast.success(`已删除 ${role.name}`);
                      }}
                    >
                      <Button size="small" danger>
                        删除
                      </Button>
                    </Popconfirm>
                  )}
                </Space>
              }
            >
              {role.description && (
                <p className="mb-3 text-sm text-text-secondary">{role.description}</p>
              )}
              <Space size={[4, 4]} wrap>
                {role.permissions.length === 0 ? (
                  <span className="text-sm text-text-muted">暂无权限</span>
                ) : (
                  <>
                    {role.permissions.slice(0, 10).map((p) => (
                      <Tag key={p.id} className="font-mono text-xs">
                        {p.code}
                      </Tag>
                    ))}
                    {role.permissions.length > 10 && (
                      <Tag>+{role.permissions.length - 10}</Tag>
                    )}
                  </>
                )}
              </Space>
            </Card>
          ))}
        </div>
      )}

      <RoleForm
        open={open}
        initial={editing}
        onClose={() => setOpen(false)}
        onSubmit={handleSubmit}
        loading={create.isPending || update.isPending}
      />
    </div>
  );
}
