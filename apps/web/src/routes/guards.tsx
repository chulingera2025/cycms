import { Navigate, Outlet } from 'react-router-dom';
import { useAuth } from '@/stores/auth';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';

export function AdminGuard() {
  const { user, loading, isAdmin } = useAuth();
  if (loading) return <LoadingSpinner />;
  if (!user || !isAdmin) return <Navigate to="/admin/login" replace />;
  return <Outlet />;
}

export function MemberGuard() {
  const { user, loading } = useAuth();
  if (loading) return <LoadingSpinner />;
  if (!user) return <Navigate to="/login" replace />;
  return <Outlet />;
}
