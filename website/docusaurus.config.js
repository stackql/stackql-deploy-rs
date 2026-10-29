// @ts-check
// The header, footer, menus and cross-site redirect routes come from the
// shared stackql/docusaurus-config, vendored to .shared-config/ at build time
// (package.json vendor-config, run by prestart/prebuild), so this site looks
// and navigates exactly like stackql.io and the provider microsites. The
// shared createConfig() assumes a provider microsite (title, URL, editUrl),
// so this site composes the shared building blocks instead; see the shared
// README, "Composing instead of createConfig".

import {themes as prismThemes} from 'prism-react-renderer';
import {
  buildNavbar,
  buildFooter,
  sharedPlugin,
  redirectRoutes,
} from './.shared-config/index.js';

const url = 'https://stackql-deploy.io';
const baseUrl = '/';
const selfUrl = `${url}${baseUrl}`;
const title = 'StackQL Deploy';
const tagline = 'Deploy and Test Cloud and SaaS Environments using StackQL';
const description =
  'stackql-deploy is a model driven, declarative, state file-less, multi-cloud infrastructure-as-code framework built on StackQL.';
const repoUrl = 'https://github.com/stackql/stackql-deploy-rs';

// The shared "stackql-deploy" item goes through the /stackql-deploy redirect
// to stackql.io, which 301s back here; on this site it is simply the home page.
const localiseDeploy = (item) =>
  item.to === '/stackql-deploy' ? {...item, to: '/'} : item;

// STACKQL>> | DEPLOY wordmark, same box as the registry wordmark on the
// provider microsites. src/theme/Logo swaps in the -mobile variants.
const deployLogo = {
  alt: 'StackQL Deploy',
  href: '/',
  src: 'img/stackql-deploy-logo.svg',
  srcDark: 'img/stackql-deploy-logo-white.svg',
};

const sharedNavbar = buildNavbar({selfUrl});
const navbar = {
  ...sharedNavbar,
  logo: {...deployLogo},
  items: sharedNavbar.items.map(localiseDeploy),
};

const sharedFooter = buildFooter({selfUrl});
const footer = {
  ...sharedFooter,
  logo: {...deployLogo},
  links: sharedFooter.links.map((column) => ({
    ...column,
    items: column.items.map(localiseDeploy),
  })),
};

/** @type {import('@docusaurus/types').Config} */
const config = {
  title,
  tagline,
  url,
  baseUrl,
  favicon: '/favicon.ico',
  staticDirectories: ['static'],
  organizationName: 'stackql',
  projectName: 'stackql-deploy-rs',
  baseUrlIssueBanner: false,

  onBrokenLinks: 'warn',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  headTags: [
    {tagName: 'link', attributes: {rel: 'preconnect', href: 'https://fonts.googleapis.com'}},
    {tagName: 'link', attributes: {rel: 'preconnect', href: 'https://fonts.gstatic.com', crossorigin: 'anonymous'}},
    {tagName: 'link', attributes: {rel: 'apple-touch-icon', sizes: '180x180', href: '/apple-touch-icon.png'}},
    {tagName: 'link', attributes: {rel: 'icon', type: 'image/png', sizes: '32x32', href: '/favicon-32x32.png'}},
    {tagName: 'link', attributes: {rel: 'icon', type: 'image/png', sizes: '16x16', href: '/favicon-16x16.png'}},
    {tagName: 'link', attributes: {rel: 'manifest', href: '/site.webmanifest'}},
    {tagName: 'link', attributes: {rel: 'mask-icon', href: '/safari-pinned-tab.svg', color: '#0f4c81'}},
  ],

  plugins: [
    '@docusaurus/plugin-ideal-image',
    // shared redirect routes (/install, /stackqldocs, /providers, ...), the
    // provider catalog for the Providers menu, and its navbar item type
    [sharedPlugin, {selfUrl}],
    '@stackql/docusaurus-plugin-structured-data',
    [
      '@stackql/docusaurus-plugin-aeo',
      {
        llmsTxt: {
          header: description,
          instanceSections: {
            'docusaurus-plugin-content-docs@default': {title: 'Documentation', order: 1},
          },
        },
      },
    ],
  ],

  presets: [
    [
      '@docusaurus/preset-classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        sitemap: {
          changefreq: 'weekly',
          priority: 0.5,
          // the shared redirect pages are not content
          ignorePatterns: ['/search', ...redirectRoutes(baseUrl, {selfUrl})],
          filename: 'sitemap.xml',
        },
        docs: {
          sidebarPath: './sidebars.js',
          routeBasePath: '/',
          sidebarCollapsible: true,
          editUrl: `${repoUrl}/tree/main/website/`,
        },
        blog: false,
        theme: {
          customCss: './src/css/global.css',
        },
      }),
    ],
  ],

  markdown: {
    mermaid: true,
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },
  themes: ['@docusaurus/theme-mermaid'],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      image: 'img/stackql-cover.png',
      metadata: [
        {name: 'description', content: description},
        {property: 'og:description', content: description},
        {name: 'robots', content: 'index,follow'},
        {name: 'keywords', content: 'stackql, stackql-deploy, sql, infrastructure as code, iac, multi-cloud, devops, cloud services, ai agents, control plane'},
        {name: 'twitter:site', content: '@stackql'},
        {name: 'twitter:creator', content: '@stackql'},
        {name: 'twitter:title', content: title},
        {name: 'twitter:description', content: description},
        {name: 'og:locale', content: 'en_US'},
        {name: 'og:site_name', content: title},
        {name: 'msapplication-TileColor', content: '#2d89ef'},
        {name: 'theme-color', content: '#ffffff'},
      ],
      // @stackql/docusaurus-plugin-structured-data (same plugin and
      // organization identity as stackql.io)
      structuredData: {
        excludedRoutes: [],
        verbose: false,
        techArticleRoutePrefixes: [],
        techArticleDocsInstances: ['default'],
        breadcrumbLinkAncestors: true,
        featuredImageDimensions: {
          width: 1200,
          height: 627,
        },
        softwareSourceCode: {
          name: 'stackql-deploy',
          codeRepository: repoUrl,
          programmingLanguage: 'Rust',
          license: 'https://opensource.org/licenses/MIT',
          runtimePlatform: 'macOS, Linux, Windows',
        },
        softwareApplication: {
          name: 'stackql-deploy',
          applicationCategory: 'DeveloperApplication',
          operatingSystem: 'macOS, Linux, Windows',
          license: 'https://opensource.org/licenses/MIT',
          downloadUrl: `${url}/getting-started`,
          isAccessibleForFree: true,
          priceCurrency: 'USD',
        },
        organization: {
          name: 'StackQL Studios',
          url: 'https://stackql.io',
          sameAs: [
            'https://twitter.com/stackql',
            'https://www.linkedin.com/company/stackql',
            'https://github.com/stackql',
            'https://www.youtube.com/@stackql',
            'https://hub.docker.com/u/stackql',
          ],
          contactPoint: {
            '@type': 'ContactPoint',
            contactType: 'customer support',
            email: 'info@stackql.io',
            telephone: '+61 3 8658 5880',
            availableLanguage: 'en',
            url: 'https://stackql.io/contact-us',
          },
          logo: {
            '@type': 'ImageObject',
            inLanguage: 'en-US',
            '@id': 'https://stackql.io/#logo',
            url: 'https://stackql.io/android-chrome-512x512.png',
            contentUrl: 'https://stackql.io/android-chrome-512x512.png',
            width: 512,
            height: 512,
            caption: 'StackQL',
          },
          address: {
            '@type': 'PostalAddress',
            addressCountry: 'AU',
            postalCode: '3000',
            streetAddress: 'Level 24, 570 Bourke Street, Melbourne, Victoria',
          },
          taxID: 'ABN 65 656 147 054',
        },
        website: {
          inLanguage: 'en-US',
        },
        webpage: {
          inLanguage: 'en-US',
        },
        breadcrumbLabelMap: {
          'cli-reference': 'CLI Command Reference',
          'template-library': 'Template Library',
          '/template-library/aws': 'AWS',
          '/template-library/azure': 'Microsoft Azure',
          '/template-library/google': 'Google Cloud Platform',
        },
      },
      docs: {
        sidebar: {
          hideable: true,
        },
      },
      navbar,
      footer,
      colorMode: {
        respectPrefersColorScheme: true,
      },
      prism: {
        theme: prismThemes.nightOwl,
        darkTheme: prismThemes.dracula,
      },
    }),
};

export default config;
