import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import { LoginForm } from '@/features/auth/LoginForm';
import { useMemberLogin } from '@/features/auth/hooks';
import { useAuth } from '@/stores/auth';

export default function MemberLoginPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { refresh } = useAuth();
  const login = useMemberLogin();

  return (
    <LoginForm
      title="会员登录"
      loading={login.isPending}
      onSubmit={async (values) => {
        await login.mutateAsync(values);
        await refresh();
        navigate(searchParams.get('redirect') ?? '/');
      }}
      footer={
        <>
          没有账号？
          <Link to="/register" className="ml-1 text-brand hover:text-brand-hover">
            注册
          </Link>
        </>
      }
    />
  );
}
