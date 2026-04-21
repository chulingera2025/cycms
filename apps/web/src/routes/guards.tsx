import { Navigate, Outlet, useLocation } from 'react-router-dom';
import { useAuth } from '@/stores/auth';
import { PageSkeleton } from '@/components/shared/PageSkeleton';

export function AdminGuard() {
  const { user, loading, isAdmin } = useAuth();
  if (loading) return <PageSkeleton variant="dashboard" />;
  if (!user || !isAdmin) return <Navigate to="/admin/login" replace />;
  return <Outlet />;
}

export function MemberGuard() {
  const { user, loading } = useAuth();
  const location = useLocation();
  if (loading) return <PageSkeleton variant="detail" />;
  if (!user) {
    const redirect = encodeURIComponent(location.pathname + location.search);
    return <Navigate to={`/login?redirect=${redirect}`} replace />;
  }
  return <Outlet />;
}
