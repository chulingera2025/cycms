import { useState } from 'react';
import { zodResolver } from '@hookform/resolvers/zod';
import { Alert, Button, Form, Input } from 'antd';
import { Controller, useForm } from 'react-hook-form';
import { Link, useNavigate } from 'react-router-dom';
import { ApiError } from '@/lib/api/client';
import { registerSchema, type RegisterInput } from '@/features/auth/schema';
import { useMemberLogin, useRegister } from '@/features/auth/hooks';
import { useAuth } from '@/stores/auth';

export default function MemberRegisterPage() {
  const navigate = useNavigate();
  const { refresh } = useAuth();
  const register = useRegister();
  const login = useMemberLogin();
  const [submitError, setSubmitError] = useState('');

  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<RegisterInput>({
    resolver: zodResolver(registerSchema),
    defaultValues: { username: '', email: '', password: '', confirmPassword: '' },
  });

  async function onSubmit(values: RegisterInput) {
    setSubmitError('');
    try {
      await register.mutateAsync({
        username: values.username,
        email: values.email,
        password: values.password,
      });
      await login.mutateAsync({
        username: values.username,
        password: values.password,
      });
      await refresh();
      navigate('/profile');
    } catch (err) {
      setSubmitError(err instanceof ApiError ? err.message : '注册失败');
    }
  }

  const busy = isSubmitting || register.isPending || login.isPending;

  return (
    <div className="grid min-h-[calc(100vh-64px)] place-items-center p-4">
      <div className="w-full max-w-[420px] rounded-lg border border-border bg-surface p-8 shadow">
        <h1 className="mb-6 text-center text-2xl font-semibold text-text">会员注册</h1>
        <Form layout="vertical" onFinish={handleSubmit(onSubmit)}>
          {submitError && (
            <Alert
              type="error"
              message={submitError}
              showIcon
              style={{ marginBottom: 16 }}
            />
          )}
          <Controller
            name="username"
            control={control}
            render={({ field }) => (
              <Form.Item
                label="用户名"
                validateStatus={errors.username ? 'error' : undefined}
                help={errors.username?.message}
              >
                <Input {...field} size="large" autoComplete="username" />
              </Form.Item>
            )}
          />
          <Controller
            name="email"
            control={control}
            render={({ field }) => (
              <Form.Item
                label="邮箱"
                validateStatus={errors.email ? 'error' : undefined}
                help={errors.email?.message}
              >
                <Input {...field} type="email" size="large" autoComplete="email" />
              </Form.Item>
            )}
          />
          <Controller
            name="password"
            control={control}
            render={({ field }) => (
              <Form.Item
                label="密码"
                validateStatus={errors.password ? 'error' : undefined}
                help={errors.password?.message}
              >
                <Input.Password {...field} size="large" autoComplete="new-password" />
              </Form.Item>
            )}
          />
          <Controller
            name="confirmPassword"
            control={control}
            render={({ field }) => (
              <Form.Item
                label="确认密码"
                validateStatus={errors.confirmPassword ? 'error' : undefined}
                help={errors.confirmPassword?.message}
              >
                <Input.Password {...field} size="large" autoComplete="new-password" />
              </Form.Item>
            )}
          />
          <Button type="primary" htmlType="submit" size="large" block loading={busy}>
            注册
          </Button>
        </Form>
        <div className="mt-4 text-center text-sm text-text-secondary">
          已有账号？
          <Link to="/login" className="ml-1 text-brand hover:text-brand-hover">
            登录
          </Link>
        </div>
      </div>
    </div>
  );
}
