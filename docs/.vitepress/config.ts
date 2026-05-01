import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "Shekere",
  description: "Visualizer for the modern age",
  head: [['link', { rel: 'icon', href: '/icon.png' }]],
  
  locales: {
    root: {
      label: 'English',
      lang: 'en'
    },
    ja: {
      label: '日本語',
      lang: 'ja',
      link: '/ja/',
      themeConfig: {
        logo: '/icon.png',
        nav: [
          { text: 'ホーム', link: '/ja/' },
          { text: 'ガイド', link: '/ja/guide/' }
        ],
        sidebar: [
          {
            text: 'ガイド',
            items: [
              { text: 'Shekereとは', link: '/ja/guide/' },
              { text: 'はじめに', link: '/ja/guide/getting-started' },
              { text: 'スケッチの書き方', link: '/ja/guide/writing-sketches' },
              { text: 'オーディオ', link: '/ja/guide/audio' },
              { text: 'MIDI', link: '/ja/guide/midi' },
              { text: 'OSC', link: '/ja/guide/osc' },
              { text: 'エフェクト', link: '/ja/guide/effects' },
              { text: 'プレイリスト', link: '/ja/guide/playlist' }
            ]
          }
        ]
      }
    }
  },

  themeConfig: {
    logo: '/icon.png',
    // Default (English) theme config
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/guide/' }
    ],

    sidebar: [
      {
        text: 'Guide',
        items: [
          { text: 'What is Shekere?', link: '/guide/' },
          { text: 'Getting Started', link: '/guide/getting-started' },
          { text: 'Writing Sketches', link: '/guide/writing-sketches' },
          { text: 'Audio', link: '/guide/audio' },
          { text: 'MIDI', link: '/guide/midi' },
          { text: 'OSC', link: '/guide/osc' },
          { text: 'Effects', link: '/guide/effects' },
          { text: 'Playlist', link: '/guide/playlist' }
        ]
      }
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/katk3n/shekere' }
    ]
  }
})
