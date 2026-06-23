import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'pc-tool 插件开发',
  description: '面向插件开发者的 pc-tool 开发文档',
  lang: 'zh-CN',

  appearance: 'dark',

  themeConfig: {
    search: {
      provider: 'local'
    },

    nav: [
      { text: '指南', link: '/index' },
      { text: '参考', link: '/manifest' },
      { text: '示例', link: '/examples' }
    ],

    sidebar: [
      {
        text: '指南',
        items: [
          { text: '介绍', link: '/index' },
          { text: '快速上手', link: '/quickstart' },
          { text: '插件类型与触发', link: '/plugin-types' },
          { text: '打包与分享', link: '/packaging' },
          { text: '安全模型', link: '/security' }
        ]
      },
      {
        text: '参考',
        items: [
          { text: 'plugin.json', link: '/manifest' },
          { text: 'host.* API', link: '/host-api' },
          { text: '能力清单', link: '/capabilities' }
        ]
      },
      {
        text: '示例',
        items: [
          { text: '完整示例', link: '/examples' }
        ]
      }
    ]
  }
})
