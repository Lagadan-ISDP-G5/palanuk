<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  // import { connectToZenoh, disconnectFromZenoh } from '../lib/stores/zenohStore';
  import '../app.css';

  onMount(() => {
    // Connect to Zenoh WebSocket bridge
    connectToZenoh('ws://localhost:8081/ws');
  });

  onDestroy(() => {
    // Cleanup on component destroy
    disconnectFromZenoh();
  });
</script>

<nav class="bg-gray-800 p-4">
  <a href="/" class="text-white mr-4">Home</a>
  <a href="/stream" class="text-white">Live Stream</a>
</nav>

<slot />