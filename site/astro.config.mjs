import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
export default defineConfig({
  site: 'https://jerrettdavis.github.io',
  base: '/BetBlocker',
  integrations: [
    starlight({
      title: 'BetBlocker',
      description: 'Open-source gambling blocking for recovery',
      logo: {
        src: './src/assets/logo.svg',
        replacesTitle: true,
      },
      favicon: '/favicon.svg',
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/JerrettDavis/BetBlocker' },
      ],
      customCss: ['./src/styles/landing.css'],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Quick Start', slug: 'getting-started' },
            { label: 'Self-Hosting Guide', slug: 'self-hosting' },
          ],
        },
        {
          label: 'Platform Guides',
          items: [
            { label: 'Windows', slug: 'platform-guides/windows' },
            { label: 'macOS', slug: 'platform-guides/macos' },
            { label: 'Linux', slug: 'platform-guides/linux' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'API Reference', slug: 'api-reference' },
            { label: 'Architecture', slug: 'architecture' },
          ],
        },
        {
          label: 'Architecture Deep Dives',
          collapsed: true,
          items: [
            { label: 'Agent Protocol', slug: 'architecture/agent-protocol' },
            { label: 'API Specification', slug: 'architecture/api-spec' },
            { label: 'Database Schema', slug: 'architecture/database-schema' },
            { label: 'Repository Structure', slug: 'architecture/repo-structure' },
            { label: 'Threat Model', slug: 'architecture/threat-model' },
          ],
        },
        {
          label: 'Community',
          items: [
            { label: 'Contributing', slug: 'contributing' },
          ],
        },
      ],
      head: [
        {
          tag: 'meta',
          attrs: {
            property: 'og:image',
            content: '/BetBlocker/og-image.png',
          },
        },
      ],
    }),
  ],
});
