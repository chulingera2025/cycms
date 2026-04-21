import { useNavigate } from 'react-router-dom';
import { LoginForm } from '@/features/auth/LoginForm';
import { useAdminLogin } from '@/features/auth/hooks';
import { useAuth } from '@/stores/auth';

export default function AdminLoginPage() {
  const navigate = useNavigate();
  const { refresh } = useAuth();
  const login = useAdminLogin();

  return (
    <LoginForm
      title="CyCMS 管理后台"
      loading={login.isPending}
      onSubmit={async (values) => {
        await login.mutateAsync(values);
        await refresh();
        navigate('/admin');
      }}
    />
  );
}
