<script>
  import { onMount, onDestroy } from 'svelte';
  import { 
    initWebSocket, 
    getWebSocketClient, 
    telemetryData, 
    connectionStatus,
    vehicleState,
    commandFeedback,
    messageLog
  } from '$lib/websocket';

  let wsClient;

  onMount(() => {
    wsClient = initWebSocket('ws://localhost:8081');
  });

  onDestroy(() => {
    if (wsClient) {
      wsClient.disconnect();
    }
  });

  function sendCommand(command) {
    if (wsClient) {
      wsClient.send({
        type: 'command',
        payload: command
      });
    }
  }
</script>

<div class="dashboard">
  <h1>UGV Dashboard</h1>
  
  <!-- Connection Status -->
  <div class="status-box">
    <h2>Connection Status</h2>
    <div class="status status-{$connectionStatus}">
      {$connectionStatus.toUpperCase()}
    </div>
  </div>

  <!-- Vehicle State -->
  <div class="state-box">
    <h2>Vehicle State</h2>
    <div class="vehicle-status vehicle-{$vehicleState.status}">
      {$vehicleState.status.toUpperCase()}
    </div>
  </div>

  <!-- Command Feedback -->
  {#if $commandFeedback}
    <div class="feedback-box feedback-{$commandFeedback.success ? 'success' : 'error'}">
      <div class="feedback-content">
        <span class="feedback-icon">{$commandFeedback.success ? '✅' : '❌'}</span>
        <span class="feedback-message">{$commandFeedback.message}</span>
        <span class="feedback-time">{$commandFeedback.timestamp}</span>
      </div>
    </div>
  {/if}

  <!-- Live Telemetry Data -->
  <div class="telemetry-box">
    <h2>Live Telemetry</h2>
    <div class="telemetry-grid">
      <div class="telemetry-item">
        <span class="label">Speed:</span>
        <span class="value">{$telemetryData.speed?.toFixed(2) || 0} m/s</span>
      </div>
      <div class="telemetry-item">
        <span class="label">Battery:</span>
        <span class="value">{$telemetryData.battery?.toFixed(1) || 0}%</span>
      </div>
      <div class="telemetry-item">
        <span class="label">Position X:</span>
        <span class="value">{$telemetryData.position?.x?.toFixed(2) || 0}</span>
      </div>
      <div class="telemetry-item">
        <span class="label">Position Y:</span>
        <span class="value">{$telemetryData.position?.y?.toFixed(2) || 0}</span>
      </div>
      <div class="telemetry-item">
        <span class="label">Heading:</span>
        <span class="value">{$telemetryData.heading?.toFixed(1) || 0}°</span>
      </div>
    </div>
  </div>

  <!-- Control Buttons -->
  <div class="controls-box">
    <h2>Controls</h2>
    <button 
      class="btn-start" 
      on:click={() => sendCommand('start')}
      disabled={$vehicleState.status === 'running'}
    >
      ▶️ Start
    </button>
    <button 
      class="btn-stop" 
      on:click={() => sendCommand('stop')}
      disabled={$vehicleState.status === 'stopped'}
    >
      ⏹️ Stop
    </button>
    <button 
      class="btn-reset" 
      on:click={() => sendCommand('reset')}
    >
      🔄 Reset
    </button>
  </div>

  <!-- Message Log -->
  <div class="log-box">
    <h2>Message Log (Last 10)</h2>
    <div class="log-container">
      {#each $messageLog.slice(-10).reverse() as msg}
        <div class="log-entry">
          <span class="log-time">{msg.time}</span>
          <span class="log-data">{JSON.stringify(msg.data)}</span>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .dashboard {
    padding: 20px;
    max-width: 1200px;
    margin: 0 auto;
    font-family: system-ui, -apple-system, sans-serif;
  }

  h1 {
    color: #333;
    margin-bottom: 30px;
  }

  h2 {
    font-size: 1.2em;
    margin: 0 0 15px 0;
    color: #555;
  }

  .status-box, .state-box, .telemetry-box, .controls-box, .log-box {
    background: white;
    border: 2px solid #ddd;
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 20px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }

  /* Feedback Box */
  .feedback-box {
    padding: 15px;
    border-radius: 8px;
    margin-bottom: 20px;
    animation: slideIn 0.3s ease-out;
  }

  .feedback-success {
    background: #d4edda;
    border: 2px solid #28a745;
  }

  .feedback-error {
    background: #f8d7da;
    border: 2px solid #dc3545;
  }

  .feedback-content {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .feedback-icon {
    font-size: 1.5em;
  }

  .feedback-message {
    flex: 1;
    font-weight: bold;
    font-size: 1.1em;
  }

  .feedback-time {
    color: #666;
    font-size: 0.9em;
  }

  @keyframes slideIn {
    from {
      transform: translateY(-20px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  /* Vehicle State */
  .vehicle-status {
    padding: 10px 20px;
    border-radius: 4px;
    font-weight: bold;
    display: inline-block;
    font-size: 1.2em;
  }

  .vehicle-stopped {
    background: #f8d7da;
    color: #721c24;
  }

  .vehicle-running {
    background: #d4edda;
    color: #155724;
  }

  /* Connection Status */
  .status {
    padding: 10px 20px;
    border-radius: 4px;
    font-weight: bold;
    display: inline-block;
  }

  .status-connected {
    background: #d4edda;
    color: #155724;
    border: 1px solid #c3e6cb;
  }

  .status-disconnected {
    background: #f8d7da;
    color: #721c24;
    border: 1px solid #f5c6cb;
  }

  /* Telemetry Grid */
  .telemetry-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 15px;
  }

  .telemetry-item {
    padding: 15px;
    background: #f8f9fa;
    border-radius: 6px;
    border-left: 4px solid #007bff;
  }

  .telemetry-item .label {
    display: block;
    font-size: 0.9em;
    color: #666;
    margin-bottom: 5px;
  }

  .telemetry-item .value {
    display: block;
    font-size: 1.5em;
    font-weight: bold;
    color: #333;
  }

  /* Control Buttons */
  .controls-box button {
    border: none;
    padding: 12px 24px;
    margin-right: 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1em;
    font-weight: bold;
    transition: all 0.2s;
  }

  .btn-start {
    background: #28a745;
    color: white;
  }

  .btn-start:hover:not(:disabled) {
    background: #218838;
  }

  .btn-stop {
    background: #dc3545;
    color: white;
  }

  .btn-stop:hover:not(:disabled) {
    background: #c82333;
  }

  .btn-reset {
    background: #ffc107;
    color: #333;
  }

  .btn-reset:hover {
    background: #e0a800;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Message Log */
  .log-container {
    max-height: 300px;
    overflow-y: auto;
    background: #f8f9fa;
    padding: 10px;
    border-radius: 4px;
  }

  .log-entry {
    padding: 8px;
    margin-bottom: 8px;
    background: white;
    border-left: 3px solid #007bff;
    font-family: monospace;
    font-size: 0.85em;
  }

  .log-time {
    color: #666;
    margin-right: 10px;
  }

  .log-data {
    color: #333;
    word-break: break-all;
  }
</style>