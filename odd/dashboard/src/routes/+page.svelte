<script lang="ts">
  import ZenohDashboard from '../lib/components/ZenohDashboard.svelte';
</script>

<div class="container mx-auto">
  <ZenohDashboard />
</div>

<style>
  :global(body) {
    background-color: #f3f4f6;
  }

  .container {
    min-height: 100vh;
    padding-top: 2rem;
  }
</style>