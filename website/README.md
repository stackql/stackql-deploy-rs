# Website

This website is built using [Docusaurus](https://docusaurus.io/), a modern static website generator.

### Installation

```
$ yarn
```

### Local Development

```
$ yarn start
```

This command starts a local development server and opens up a browser window. Most changes are reflected live without having to restart the server.

### Build

```
$ yarn build
```

This command generates static content into the `build` directory and can be served using any static contents hosting service.

### Shared StackQL chrome

The navbar, footer, menus and cross-site redirect routes are shared with
[stackql.io](https://stackql.io) and the provider microsites through
[stackql/docusaurus-config](https://github.com/stackql/docusaurus-config).
It is vendored, not installed: `yarn start` and `yarn build` first run
`yarn vendor-config`, which shallow-clones its `main` branch into the
gitignored `.shared-config/` folder, so `git` and network access are needed
to build. The Providers menu is generated from `https://stackql.io/providers.json`
at build time; set `STACKQL_PROVIDER_CATALOG` to a local copy to build offline.

`docusaurus.config.js` composes the shared building blocks (`buildNavbar`,
`buildFooter`, `sharedPlugin`, `redirectRoutes`) rather than calling
`createConfig`, which is specific to the provider microsites. Site-local pieces:

- the `STACKQL>> | DEPLOY` wordmark (`static/img/stackql-deploy-logo*.svg`),
  sized to the same 320x40 box as the registry wordmark on the provider sites
- the sidebar, which opens with the shared "Back to StackQL Docs" link
  (`/stackqldocs`, styled by `.sidebar-back-link` in `src/css/global.css`)
- `@stackql/docusaurus-plugin-structured-data` (config under
  `themeConfig.structuredData`) and `@stackql/docusaurus-plugin-aeo`
  (`.md` companions, `llms.txt`, Ask AI button), the same plugins as stackql.io

Do not add pages under `src/pages` at the shared redirect paths (`/install`,
`/stackqldocs`, `/providers`, `/blog`, `/contact-us`, ...); the shared plugin
registers those routes.

### Deployment

Using SSH:

```
$ USE_SSH=true yarn deploy
```

Not using SSH:

```
$ GIT_USER=<Your GitHub username> yarn deploy
```

If you are using GitHub pages for hosting, this command is a convenient way to build the website and push to the `gh-pages` branch.
