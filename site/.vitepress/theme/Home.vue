<script setup lang="ts">
import { ref } from 'vue'
import { withBase } from 'vitepress'
import { RELEASE_URL, REPO_URL } from './links'

const levels = [
  { name: 'The Badlands', status: 'Playable', text: 'Mossy cliffs below a ruined city, from a violet dusk to a burning night. Sentry walkers, Crawler spider-bots, Drones and rolling Mines, then URSA-9000, a bear-shaped war mech.' },
  { name: 'Solana Trenches', status: 'Planned', text: 'A neon casino strip full of hustlers.' },
  { name: 'Ethereum City', status: 'Planned', text: 'A futuristic city of gas drones and MEV bots.' },
]

const shots = [
  { file: 'level-start', caption: 'Level 1: The Badlands' },
  { file: 'hover', caption: 'Jetpack hover over the ruined skyline' },
  { file: 'dialog-1', caption: 'URSA-9000 offers a deal' },
  { file: 'dialog-3', caption: '“Hold this.”' },
  { file: 'map-badlands', caption: 'The world map' },
  { file: 'title', caption: 'Title screen' },
]
const open = ref<string | null>(null)
</script>

<template>
  <section class="band band-primary hero">
    <div class="container hero-grid">
      <div>
        <h1 class="main-title text-meme">
          <span class="text-highlight">Diamond Hands</span>, a wassie with an oversized laser cannon
        </h1>
        <p class="lead text-meme">For the Game Boy Advance. Blast through the bears' cyber bots, left to right.</p>
        <div class="actions">
          <a class="btn btn-secondary" :href="RELEASE_URL" target="_blank" rel="noopener">Download ROM</a>
          <a class="btn btn-light" :href="REPO_URL" target="_blank" rel="noopener">View source</a>
        </div>
      </div>
      <img class="cover" :src="withBase('/cover.png')" alt="Diamond Hands box art" width="900" height="900" />
    </div>
  </section>

  <section id="concept" class="band band-light">
    <div class="container">
      <h2 class="text-meme">Concept</h2>
      <p class="intro">
        In a dystopian future the bears have built machines. A wassie in a red poncho, hauling an oversized
        laser cannon, blasts its way left to right through their cyber bots. Every level ends with a boss.
      </p>
      <div class="cards">
        <article v-for="(l, i) in levels" :key="l.name" class="paper card">
          <span class="chip" :class="{ on: l.status === 'Playable' }">{{ l.status }}</span>
          <h3>{{ i + 1 }}. {{ l.name }}</h3>
          <p>{{ l.text }}</p>
        </article>
      </div>
    </div>
  </section>

  <section id="screenshots" class="band band-primary">
    <div class="container">
      <h2 class="text-meme">Screenshots</h2>
      <div class="gallery">
        <figure v-for="s in shots" :key="s.file" class="paper" @click="open = s.file">
          <img :src="withBase(`/shots/${s.file}.png`)" :alt="s.caption" loading="lazy" width="720" height="480" />
          <figcaption>{{ s.caption }}</figcaption>
        </figure>
      </div>
    </div>
  </section>

  <section id="download" class="band band-secondary">
    <div class="container center">
      <h2 class="text-meme">Play it</h2>
      <p class="intro text-meme">
        Grab <code>diamond-hands.gba</code> from the latest GitHub Release. It runs on a flash cart and in any
        GBA emulator, such as mGBA.
      </p>
      <a class="btn btn-light btn-lg" :href="RELEASE_URL" target="_blank" rel="noopener">Download ROM</a>
    </div>
  </section>

  <div v-if="open" class="lightbox" @click="open = null">
    <img :src="withBase(`/shots/${open}.png`)" alt="" />
  </div>
</template>
