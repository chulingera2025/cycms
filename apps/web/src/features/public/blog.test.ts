import { describe, expect, it } from 'vitest';
import type { ContentEntry } from '@/types';
import {
  toBlogPostDetail,
  toBlogPostSummary,
  toBlogSiteSettings,
} from './blog';

const basePost: ContentEntry = {
  id: 'post-1',
  content_type_id: 'ct-post',
  content_type_api_id: 'post',
  slug: 'hello-cycms',
  status: 'published',
  current_version_id: 'rev-1',
  published_version_id: 'rev-1',
  fields: {
    title: 'Hello CyCMS',
    excerpt: 'A short intro',
    cover_image: 'media-1',
    body: '# Hello',
    featured: true,
    seo_title: 'SEO Hello',
    seo_description: 'SEO description',
  },
  created_by: 'user-1',
  updated_by: 'user-1',
  created_at: '2026-04-22T00:00:00Z',
  updated_at: '2026-04-22T00:00:00Z',
  published_at: '2026-04-22T01:00:00Z',
};

describe('public blog adapters', () => {
  it('maps a content entry into blog summary fields', () => {
    expect(toBlogPostSummary(basePost)).toEqual({
      id: 'post-1',
      slug: 'hello-cycms',
      title: 'Hello CyCMS',
      excerpt: 'A short intro',
      coverImageId: 'media-1',
      featured: true,
      publishedAt: '2026-04-22T01:00:00Z',
    });
  });

  it('maps populated relations into post detail aggregates', () => {
    const detail = toBlogPostDetail({
      ...basePost,
      populated: {
        categories: [
          {
            ...basePost,
            id: 'category-1',
            content_type_api_id: 'category',
            slug: 'engineering',
            fields: { name: 'Engineering', description: 'Posts about building' },
          },
        ],
        tags: [
          {
            ...basePost,
            id: 'tag-1',
            content_type_api_id: 'tag',
            slug: 'rust',
            fields: { name: 'Rust', description: 'Rust language' },
          },
        ],
      },
    });

    expect(detail.categories[0]).toMatchObject({ slug: 'engineering', name: 'Engineering' });
    expect(detail.tags[0]).toMatchObject({ slug: 'rust', name: 'Rust' });
    expect(detail.body).toBe('# Hello');
  });

  it('maps site settings and featured posts with fallback defaults', () => {
    const settings = toBlogSiteSettings({
      ...basePost,
      id: 'settings-1',
      content_type_api_id: 'site_settings',
      slug: undefined,
      fields: {
        site_name: 'CyCMS Blog',
        tagline: 'Structured publishing',
        logo: 'media-logo',
        hero_title: 'Build with content',
        hero_subtitle: 'A calmer publishing stack',
        footer_text: 'All rights reserved.',
      },
      populated: {
        featured_posts: [basePost],
      },
    });

    expect(settings).toMatchObject({
      siteName: 'CyCMS Blog',
      tagline: 'Structured publishing',
      logoId: 'media-logo',
      heroTitle: 'Build with content',
      heroSubtitle: 'A calmer publishing stack',
      footerText: 'All rights reserved.',
    });
    expect(settings?.featuredPosts[0]).toMatchObject({ slug: 'hello-cycms', title: 'Hello CyCMS' });
    expect(toBlogSiteSettings(null)).toBeNull();
  });
});