// @ts-check
// `@type` JSDoc annotations allow editor autocompletion and type checking
// (when a type is always passed). Optional: run `npm run build` to validate.

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Grainlify',
  tagline: 'Grant execution infrastructure for open-source ecosystems',
  favicon: 'img/logo.svg',

  url: 'https://grainlify.io',
  baseUrl: '/',

  organizationName: 'grainlify',
  projectName: 'grainlify',

  onBrokenLinks: 'warn',
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      '@docusaurus/preset-classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          routeBasePath: '/',
          sidebarPath: './sidebars.js',
          editUrl: undefined,
          showLastUpdateTime: true,
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      }),
    ],
  ],

  plugins: [
    [
      '@cmfcmf/docusaurus-search-local',
      {
        indexDocs: true,
        indexBlog: false,
        language: 'en',
      },
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      navbar: {
        title: 'Grainlify',
        logo: {
          alt: 'Grainlify',
          src: 'img/logo.svg',
          srcDark: 'img/logo.svg',
        },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'docsSidebar',
            position: 'left',
            label: 'Docs',
          },
          {
            type: 'search',
            position: 'right',
          },
          {
            href: 'https://github.com/grainlify/grainlify',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Docs',
            items: [
              { label: 'Introduction', to: '/' },
              { label: 'What is Grainlify?', to: '/what-is-grainlify' },
              { label: 'How it works', to: '/how-it-works' },
            ],
          },
          {
            title: 'Community',
            items: [
              { label: 'GitHub', href: 'https://github.com/grainlify/grainlify' },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} Grainlify. Built with Docusaurus.`,
      },
      colorMode: {
        defaultMode: 'light',
        respectPrefersColorScheme: false,
      },
      docs: {
        sidebar: {
          hideable: true,
        },
      },
    }),
};

export default config;
