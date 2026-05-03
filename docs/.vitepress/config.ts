import { defineConfig } from 'vitepress'
import pkg from '../../package.json'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  base: '/shekere/',
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
          { text: 'ガイド', link: '/ja/guide/' },
          {
            text: `v${pkg.version}`,
            items: [
              { text: 'リリースノート', link: 'https://github.com/katk3n/shekere/releases' }
            ]
          }
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
              { text: 'シェーダー (TSL)', link: '/ja/guide/shaders' },
              { text: 'プレイリスト', link: '/ja/guide/playlist' }
            ]
          }
        ]
      }
    }
  },

  themeConfig: {
    search: {
      provider: 'local'
    },
    logo: '/icon.png',
    // Default (English) theme config
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/guide/' },
      {
        text: `v${pkg.version}`,
        items: [
          { text: 'Release Notes', link: 'https://github.com/katk3n/shekere/releases' }
        ]
      }
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
          { text: 'Shaders (TSL)', link: '/guide/shaders' },
          { text: 'Playlist', link: '/guide/playlist' }
        ]
      }
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/katk3n/shekere' }
    ]
  }
})
