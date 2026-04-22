import { useState } from 'react';
import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom';
import { Drawer } from 'antd';
import { LogIn, Menu as MenuIcon, Search, UserCircle } from 'lucide-react';
import { useBlogSiteSettings } from '@/features/public/hooks';
import { useMedia } from '@/features/media/hooks';
import { useTranslation } from 'react-i18next';
import { useAuth } from '@/stores/auth';
import { ThemeSwitcher } from '@/components/shared/ThemeSwitcher';
import { resolveMediaUrl } from '@/utils/format';

export default function PublicLayout() {
  const { user } = useAuth();
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useTranslation(['public', 'common']);
  const { data: siteSettings } = useBlogSiteSettings();
  const { data: logo } = useMedia(siteSettings?.logoId);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  const siteName = siteSettings?.siteName ?? t('app.name', { ns: 'common' });
  const footerText = siteSettings?.footerText ?? `${siteName}. All rights reserved.`;

  const loginHref = `/login?redirect=${encodeURIComponent(location.pathname + location.search)}`;

  function handleSearch(e: React.FormEvent) {
    e.preventDefault();
    const q = searchQuery.trim();
    if (!q) return;
    navigate(`/search?q=${encodeURIComponent(q)}`);
    setSearchOpen(false);
    setSearchQuery('');
  }

  const navLinks = (
    <>
      <Link
        to="/"
        className="text-sm text-text-secondary no-underline transition-colors hover:text-text"
      >
        {t('nav.home')}
      </Link>
      <Link
        to="/blog"
        className="text-sm text-text-secondary no-underline transition-colors hover:text-text"
      >
        {t('nav.content')}
      </Link>
    </>
  );

  const authLinks = user ? (
    <>
      <Link
        to="/profile"
        className="inline-flex items-center gap-1 text-sm text-text no-underline transition-colors hover:text-brand"
      >
        <UserCircle size={16} />
        <span>{user.username}</span>
      </Link>
      <Link
        to="/admin"
        className="inline-flex items-center rounded border border-border bg-surface px-3 py-1 text-sm text-text-secondary no-underline transition-colors hover:border-brand hover:text-brand"
      >
        管理后台
      </Link>
    </>
  ) : (
    <>
      <Link
        to={loginHref}
        className="inline-flex items-center gap-1 text-sm text-text-secondary no-underline transition-colors hover:text-text"
      >
        <LogIn size={14} />
        {t('nav.login')}
      </Link>
      <Link
        to="/register"
        className="rounded bg-brand px-3 py-1 text-sm text-white no-underline transition-colors hover:bg-brand-hover"
      >
        {t('nav.register')}
      </Link>
    </>
  );

  return (
    <div className="flex min-h-screen flex-col bg-bg text-text">
      <header className="sticky top-0 z-20 border-b border-border bg-surface/80 backdrop-blur">
        <div className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-3 px-4">
          <div className="flex items-center gap-6">
            <Link
              to="/"
              className="flex items-center gap-2 text-text no-underline hover:text-text"
            >
              {logo ? (
                <img
                  src={resolveMediaUrl(logo.storage_path)}
                  alt={siteName}
                  className="h-7 w-7 rounded object-cover"
                />
              ) : (
                <span className="grid h-7 w-7 place-items-center rounded bg-brand text-sm font-bold text-white">
                  {siteName.slice(0, 1).toUpperCase()}
                </span>
              )}
              <span className="text-base font-semibold tracking-wide">
                {siteName}
              </span>
            </Link>
            <nav className="hidden items-center gap-5 md:flex">{navLinks}</nav>
          </div>

          <div className="flex items-center gap-1">
            {searchOpen ? (
              <form onSubmit={handleSearch} className="flex items-center">
                <input
                  autoFocus
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  onBlur={() => setTimeout(() => setSearchOpen(false), 120)}
                  placeholder={t('actions.search', { ns: 'common' })}
                  className="h-8 w-40 rounded border border-border bg-surface px-2 text-sm text-text outline-none focus:border-brand"
                />
              </form>
            ) : (
              <button
                type="button"
                aria-label={t('actions.search', { ns: 'common' })}
                onClick={() => setSearchOpen(true)}
                className="inline-flex h-8 w-8 items-center justify-center rounded text-text-secondary transition-colors hover:bg-surface-alt hover:text-text"
              >
                <Search size={16} />
              </button>
            )}
            <ThemeSwitcher />
            <div className="hidden items-center gap-2 md:flex">{authLinks}</div>
            <button
              type="button"
              aria-label="菜单"
              className="inline-flex h-8 w-8 items-center justify-center rounded text-text-secondary transition-colors hover:bg-surface-alt hover:text-text md:hidden"
              onClick={() => setDrawerOpen(true)}
            >
              <MenuIcon size={18} />
            </button>
          </div>
        </div>
      </header>

      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        placement="right"
        width={260}
        title={siteName}
      >
        <div className="flex flex-col gap-5">
          <nav className="flex flex-col gap-3">{navLinks}</nav>
          <div className="flex flex-col gap-3 border-t border-border pt-4">{authLinks}</div>
        </div>
      </Drawer>

      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-8">
        <Outlet />
      </main>

      <footer className="border-t border-border bg-surface">
        <div className="mx-auto max-w-6xl px-4 py-6 text-center text-sm text-text-muted">
          &copy; {new Date().getFullYear()} {footerText}
        </div>
      </footer>
    </div>
  );
}
