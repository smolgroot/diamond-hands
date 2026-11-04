<script setup lang="ts">
import { withBase } from 'vitepress'
import { RELEASE_URL } from './links'
</script>

<template>
  <header class="appbar">
    <div class="container bar">
      <a class="brand" :href="withBase('/')" aria-label="Diamond Hands">
        <img :src="withBase('/logo.png')" alt="" width="52" height="52" />
        <span class="text-meme">Diamond Hands</span>
      </a>
      <nav>
        <a :href="withBase('/#concept')" class="text-meme">Concept</a>
        <a :href="withBase('/#screenshots')" class="text-meme">Screenshots</a>
        <a :href="withBase('/controls')" class="text-meme">Controls</a>
      </nav>
      <a class="btn btn-light" :href="RELEASE_URL" target="_blank" rel="noopener">Download ROM</a>
    </div>
  </header>
</template>
