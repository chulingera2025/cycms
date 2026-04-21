import { useState, type ReactNode } from 'react';
import { zodResolver } from '@hookform/resolvers/zod';
import { Alert, Button, Form, Input } from 'antd';
import { Controller, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { ApiError } from '@/lib/api/client';
import { loginSchema, type LoginInput } from './schema';

interface Props {
  title: string;
  submitLabel?: string;
  loading?: boolean;
  footer?: ReactNode;
  onSubmit: (values: LoginInput) => Promise<void>;
}

export function LoginForm({ title, submitLabel, loading, footer, onSubmit }: Props) {
  const { t } = useTranslation(['auth', 'common']);
  const [submitError, setSubmitError] = useState('');

  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<LoginInput>({
    resolver: zodResolver(loginSchema),
    defaultValues: { username: '', password: '' },
  });

  async function submit(values: LoginInput) {
    setSubmitError('');
    try {
      await onSubmit(values);
    } catch (err) {
      setSubmitError(err instanceof ApiError ? err.message : t('login.failed'));
    }
  }

  return (
    <div className="grid min-h-[calc(100vh-64px)] place-items-center bg-bg p-4">
      <div className="w-full max-w-[420px] rounded-lg border border-border bg-surface p-8 shadow">
        <h1 className="mb-6 text-center text-2xl font-semibold text-text">{title}</h1>
        <Form layout="vertical" onFinish={handleSubmit(submit)}>
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
                label={t('login.username')}
                validateStatus={errors.username ? 'error' : undefined}
                help={errors.username?.message}
              >
                <Input
                  {...field}
                  size="large"
                  autoComplete="username"
                  placeholder={t('login.username')}
                />
              </Form.Item>
            )}
          />
          <Controller
            name="password"
            control={control}
            render={({ field }) => (
              <Form.Item
                label={t('login.password')}
                validateStatus={errors.password ? 'error' : undefined}
                help={errors.password?.message}
              >
                <Input.Password
                  {...field}
                  size="large"
                  autoComplete="current-password"
                  placeholder={t('login.password')}
                />
              </Form.Item>
            )}
          />
          <Button
            type="primary"
            htmlType="submit"
            size="large"
            block
            loading={loading || isSubmitting}
          >
            {submitLabel ?? t('login.submit')}
          </Button>
        </Form>
        {footer && <div className="mt-4 text-center text-sm text-text-secondary">{footer}</div>}
      </div>
    </div>
  );
}
