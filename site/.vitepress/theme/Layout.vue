<script setup lang="ts">
import { Content, useData, withBase } from 'vitepress'
import AppBar from './AppBar.vue'
import Home from './Home.vue'

const { frontmatter } = useData()
</script>

<template>
  <AppBar />
  <main>
    <Home v-if="frontmatter.layout === 'home'" />
    <section v-else class="band band-primary">
      <div class="container prose paper"><Content /></div>
    </section>
  </main>
  <footer class="band band-dark">
    <div class="container">
      <p>A <a href="https://wassie.wtf" target="_blank" rel="noopener">wassie.wtf</a> homebrew game. Built with Rust and
        <a href="https://agbrs.dev" target="_blank" rel="noopener">agb</a>.</p>
      <p><a href="https://github.com/smolgroot/diamond-hands" target="_blank" rel="noopener">Source on GitHub</a> ·
        <a :href="withBase('/controls')">Controls</a></p>
      <p class="fine">Not affiliated with Nintendo.</p>
    </div>
  </footer>
</template>
