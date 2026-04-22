import type { ContentEntry } from '@/types';

export interface BlogCategory {
  id: string;
  slug: string;
  name: string;
  description?: string;
  coverImageId?: string;
  seoTitle?: string;
  seoDescription?: string;
}

export interface BlogTag {
  id: string;
  slug: string;
  name: string;
  description?: string;
}

export interface BlogPostSummary {
  id: string;
  slug: string;
  title: string;
  excerpt?: string;
  coverImageId?: string;
  featured: boolean;
  publishedAt?: string;
}

export interface BlogPostDetail extends BlogPostSummary {
  body: string;
  categories: BlogCategory[];
  tags: BlogTag[];
  seoTitle?: string;
  seoDescription?: string;
}

export interface BlogSiteSettings {
  id: string;
  siteName: string;
  tagline?: string;
  logoId?: string;
  heroTitle?: string;
  heroSubtitle?: string;
  footerText?: string;
  featuredPosts: BlogPostSummary[];
}

function readString(fields: Record<string, unknown>, key: string): string | undefined {
  const value = fields[key];
  return typeof value === 'string' && value.trim() !== '' ? value : undefined;
}

function readBoolean(fields: Record<string, unknown>, key: string): boolean {
  return fields[key] === true;
}

function fallbackSlug(entry: ContentEntry): string {
  return entry.slug ?? entry.id;
}

export function toBlogCategory(entry: ContentEntry): BlogCategory {
  return {
    id: entry.id,
    slug: fallbackSlug(entry),
    name: readString(entry.fields, 'name') ?? fallbackSlug(entry),
    description: readString(entry.fields, 'description'),
    coverImageId: readString(entry.fields, 'cover_image'),
    seoTitle: readString(entry.fields, 'seo_title'),
    seoDescription: readString(entry.fields, 'seo_description'),
  };
}

export function toBlogTag(entry: ContentEntry): BlogTag {
  return {
    id: entry.id,
    slug: fallbackSlug(entry),
    name: readString(entry.fields, 'name') ?? fallbackSlug(entry),
    description: readString(entry.fields, 'description'),
  };
}

export function toBlogPostSummary(entry: ContentEntry): BlogPostSummary {
  return {
    id: entry.id,
    slug: fallbackSlug(entry),
    title: readString(entry.fields, 'title') ?? fallbackSlug(entry),
    excerpt: readString(entry.fields, 'excerpt'),
    coverImageId: readString(entry.fields, 'cover_image'),
    featured: readBoolean(entry.fields, 'featured'),
    publishedAt: entry.published_at,
  };
}

export function toBlogPostDetail(entry: ContentEntry): BlogPostDetail {
  const categories = (entry.populated?.categories ?? []).map(toBlogCategory);
  const tags = (entry.populated?.tags ?? []).map(toBlogTag);

  return {
    ...toBlogPostSummary(entry),
    body: readString(entry.fields, 'body') ?? '',
    categories,
    tags,
    seoTitle: readString(entry.fields, 'seo_title'),
    seoDescription: readString(entry.fields, 'seo_description'),
  };
}

export function toBlogSiteSettings(entry: ContentEntry | null | undefined): BlogSiteSettings | null {
  if (!entry) {
    return null;
  }

  return {
    id: entry.id,
    siteName: readString(entry.fields, 'site_name') ?? 'CyCMS',
    tagline: readString(entry.fields, 'tagline'),
    logoId: readString(entry.fields, 'logo'),
    heroTitle: readString(entry.fields, 'hero_title'),
    heroSubtitle: readString(entry.fields, 'hero_subtitle'),
    footerText: readString(entry.fields, 'footer_text'),
    featuredPosts: (entry.populated?.featured_posts ?? []).map(toBlogPostSummary),
  };
}