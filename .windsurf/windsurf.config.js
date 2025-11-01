// @ts-check
import { defineConfig } from 'windsurf';

export default defineConfig({
  title: '项目文档',
  description: '项目文档描述',
  themeConfig: {
    nav: [
      { text: '指南', link: '/guide/' },
      { text: '参考', link: '/reference/' },
      { text: 'GitHub', link: 'https://github.com/your-org/your-repo' },
    ],
    sidebar: {
      '/guide/': [
        {
          text: '指南',
          items: [
            { text: '快速开始', link: '/guide/getting-started' },
            { text: '核心概念', link: '/guide/concepts' },
          ],
        },
      ],
      '/reference/': [
        {
          text: 'API 参考',
          items: [
            { text: '数据库', link: '/reference/database' },
            { text: '查询语言', link: '/reference/query' },
          ],
        },
      ],
    },
  },
  markdown: {
    theme: 'github-dark',
  },
});
