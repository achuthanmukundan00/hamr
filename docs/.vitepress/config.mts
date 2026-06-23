import { defineConfig } from 'vitepress';

export default defineConfig({
  title: 'Hamr',
  description: 'Local-first coding agent.',
  base: '/',
  sitemap: { hostname: 'https://hamr-6j6.pages.dev' },
  cleanUrls: true,
  head: [['link', { rel: 'icon', type: 'image/svg+xml', href: '/logo.svg' }]],
  themeConfig: {
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Parsers', link: '/guide/tool-call-parsing' },
      { text: 'Providers', link: '/guide/providers' },
      { text: 'Relay', link: '/guide/relay' },
      { text: 'Fork', link: '/guide/fork-lineage' },
      { text: 'GitHub', link: 'https://github.com/skaft-software/hamr' },
    ],
    sidebar: [
      {
        text: 'Getting Started',
        items: [
          { text: 'Overview', link: '/' },
          { text: 'Quick Start', link: '/guide/getting-started' },
          { text: 'Configuration', link: '/guide/configuration' },
        ],
      },
      {
        text: 'Running Models',
        items: [
          { text: 'Providers', link: '/guide/providers' },
          { text: 'Relay Setup', link: '/guide/relay' },
          { text: 'Tool-Call Parsing', link: '/guide/tool-call-parsing' },
          { text: 'Compatibility Reports', link: '/guide/compatibility' },
        ],
      },
      {
        text: 'Using Hamr',
        items: [
          { text: 'Commands & TUI', link: '/guide/commands' },
          { text: 'Agent Loop', link: '/guide/agent-loop' },
          { text: 'Tool Reference', link: '/guide/tools' },
          { text: 'Skills', link: '/guide/skills' },
          { text: 'Sessions', link: '/guide/sessions' },
        ],
      },
      {
        text: 'Reference',
        items: [
          { text: 'Architecture', link: '/guide/architecture' },
          { text: 'Runtime Architecture', link: '/architecture/runtime' },
          { text: 'Fork Lineage', link: '/guide/fork-lineage' },
          { text: 'SDK API', link: '/guide/sdk-api' },
          { text: 'Embedding Hamr', link: '/guide/consuming-hamr' },
          { text: 'Extensions & MCP', link: '/guide/extensions' },
          { text: 'Safety & Context', link: '/guide/safety-context' },
          { text: 'Development', link: '/guide/development' },
        ],
      },
    ],
    search: { provider: 'local' },
    socialLinks: [{ icon: 'github', link: 'https://github.com/skaft-software/hamr' }],
    footer: {
      message: 'Skaft Software · MIT License',
      copyright: '© 2026 Skaft Software',
    },
  },
});
