import { useState } from 'react';
import { zodResolver } from '@hookform/resolvers/zod';
import {
  Alert,
  Button,
  Descriptions,
  Form,
  Input,
  Space,
  Tabs,
  Tag,
  Typography,
} from 'antd';
import { Controller, useForm } from 'react-hook-form';
import { z } from 'zod';
import { LogOut } from 'lucide-react';
import { ApiError } from '@/lib/api/client';
import { usersApi } from '@/lib/api';
import { PageSkeleton } from '@/components/shared/PageSkeleton';
import { toast } from '@/lib/toast';
import { useAuth } from '@/stores/auth';
import { formatDateTime } from '@/utils/format';

const profileSchema = z.object({
  email: z.string().email('邮箱格式不正确'),
});

const passwordSchema = z
  .object({
    password: z.string().min(8, '密码长度至少 8 位'),
    confirmPassword: z.string(),
  })
  .refine((d) => d.password === d.confirmPassword, {
    path: ['confirmPassword'],
    message: '两次输入的密码不一致',
  });

type ProfileInput = z.infer<typeof profileSchema>;
type PasswordInput = z.infer<typeof passwordSchema>;

function ProfileForm() {
  const { user, refresh } = useAuth();
  const [error, setError] = useState('');
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<ProfileInput>({
    resolver: zodResolver(profileSchema),
    defaultValues: { email: user?.email ?? '' },
  });

  async function onSubmit(values: ProfileInput) {
    setError('');
    try {
      if (!user) return;
      await usersApi.update(user.id, { email: values.email });
      await refresh();
      toast.success('资料已更新');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    }
  }

  if (!user) return null;

  return (
    <div>
      <Descriptions
        column={1}
        bordered
        size="middle"
        items={[
          { key: 'username', label: '用户名', children: user.username },
          {
            key: 'roles',
            label: '角色',
            children:
              user.roles.length === 0 ? (
                <Typography.Text type="secondary">无</Typography.Text>
              ) : (
                <Space size={4} wrap>
                  {user.roles.map((r) => (
                    <Tag key={r}>{r}</Tag>
                  ))}
                </Space>
              ),
          },
          {
            key: 'created_at',
            label: '注册时间',
            children: formatDateTime(user.created_at),
          },
        ]}
      />

      <Typography.Title level={5} style={{ marginTop: 24 }}>
        修改邮箱
      </Typography.Title>
      <Form layout="vertical" onFinish={handleSubmit(onSubmit)} style={{ maxWidth: 420 }}>
        {error && (
          <Alert type="error" message={error} showIcon style={{ marginBottom: 16 }} />
        )}
        <Controller
          name="email"
          control={control}
          render={({ field }) => (
            <Form.Item
              label="邮箱"
              validateStatus={errors.email ? 'error' : undefined}
              help={errors.email?.message}
            >
              <Input {...field} type="email" autoComplete="email" />
            </Form.Item>
          )}
        />
        <Button type="primary" htmlType="submit" loading={isSubmitting}>
          保存
        </Button>
      </Form>
    </div>
  );
}

function PasswordForm() {
  const { user } = useAuth();
  const [error, setError] = useState('');
  const {
    control,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<PasswordInput>({
    resolver: zodResolver(passwordSchema),
    defaultValues: { password: '', confirmPassword: '' },
  });

  async function onSubmit(values: PasswordInput) {
    setError('');
    try {
      if (!user) return;
      await usersApi.update(user.id, { password: values.password });
      toast.success('密码已更新');
      reset({ password: '', confirmPassword: '' });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    }
  }

  return (
    <Form layout="vertical" onFinish={handleSubmit(onSubmit)} style={{ maxWidth: 420 }}>
      {error && (
        <Alert type="error" message={error} showIcon style={{ marginBottom: 16 }} />
      )}
      <Controller
        name="password"
        control={control}
        render={({ field }) => (
          <Form.Item
            label="新密码"
            validateStatus={errors.password ? 'error' : undefined}
            help={errors.password?.message}
          >
            <Input.Password {...field} autoComplete="new-password" />
          </Form.Item>
        )}
      />
      <Controller
        name="confirmPassword"
        control={control}
        render={({ field }) => (
          <Form.Item
            label="确认新密码"
            validateStatus={errors.confirmPassword ? 'error' : undefined}
            help={errors.confirmPassword?.message}
          >
            <Input.Password {...field} autoComplete="new-password" />
          </Form.Item>
        )}
      />
      <Button type="primary" htmlType="submit" loading={isSubmitting}>
        更新密码
      </Button>
    </Form>
  );
}

export default function ProfilePage() {
  const { user, logout } = useAuth();

  if (!user) return <PageSkeleton variant="detail" />;

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <Typography.Title level={2} style={{ margin: 0 }}>
          个人中心
        </Typography.Title>
        <Button danger icon={<LogOut size={14} />} onClick={logout}>
          退出登录
        </Button>
      </div>
      <Tabs
        defaultActiveKey="profile"
        items={[
          { key: 'profile', label: '个人资料', children: <ProfileForm /> },
          { key: 'security', label: '安全设置', children: <PasswordForm /> },
        ]}
      />
    </div>
  );
}
