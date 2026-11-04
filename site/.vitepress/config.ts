import { defineConfig } from 'vitepress'

const base = process.env.SITE_BASE ?? '/diamond-hands/'

export default defineConfig({
  base,
  title: 'Diamond Hands',
  description: 'A side-scrolling action platformer for the Game Boy Advance. A wassie, an oversized laser cannon, and a lot of angry bear bots.',
  cleanUrls: true,
  head: [
    ['link', { rel: 'icon', href: `${base}logo.png` }],
    ['meta', { property: 'og:title', content: 'Diamond Hands — a GBA homebrew game' }],
    ['meta', { property: 'og:image', content: `${base}cover.png` }],
    ['meta', { name: 'theme-color', content: '#bb90e9' }],
  ],
})
