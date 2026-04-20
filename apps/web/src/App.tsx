import { RouterProvider } from 'react-router-dom';
import { ErrorBoundary } from '@/components/shared/ErrorBoundary';
import { router } from '@/routes';

export default function App() {
  return (
    <ErrorBoundary>
      <RouterProvider router={router} />
    </ErrorBoundary>
  );
}
