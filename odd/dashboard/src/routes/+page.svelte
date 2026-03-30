<script>
  import { onMount, onDestroy } from 'svelte';
  import {
    initWebSocket,
    getWebSocketClient,
    telemetryData,
    connectionStatus,
    vehicleState,
    commandFeedback,
    messageLog,
    motorTotals
  } from '$lib/stores/zenohStore';

  const MAX_RPM = 600; // Maximum motor RPM — encoder speed is normalized 0‑1
  const PI_IP = 'raspberrypi.local'; // Change to your Pi's IP or hostname if needed
  const CAMERA_URL = `http://${PI_IP}:8889/camera/`;

  let wsClient;

  // Destructure telemetryData so Svelte tracks every field as an explicit
  // reactive dependency. Using ?. optional chaining directly inside $: blocks
  // can silently miss updates for individual fields like load_current_ma.
  $: ({
    battery          = 0,
    power_mw         = 0,
    power_w          = 0,
    load_current_ma  = 0,
    bus_voltage_mv   = 0,
    shunt_voltage_mv = 0,
    distance         = 0,
  } = $telemetryData);

  $: robotData = {
    battery:      Math.round(battery),
    power:        power_mw.toFixed(1),         // mW  — raw from bridge
    current:      load_current_ma.toFixed(1),  // mA  — raw from bridge
    voltage:      bus_voltage_mv.toFixed(0),   // mV  — raw from bridge
    shunt:        shunt_voltage_mv.toFixed(2), // mV  — raw from bridge
    totalEnergy:  power_w.toFixed(3),          // W   — pre-derived by bridge
    distance:     distance.toFixed(2),
    efficiency:   calculateEfficiency(),
    averageSpeed: Math.round(($vehicleState?.status?.includes('moving')) ? 10.8 : 0),
    travelTime:   calculateTravelTime(),
  };

  function calculateEfficiency() {
    // Uses destructured 'distance' and 'power_mw' from the reactive block above
    if (distance > 0 && power_mw > 0) {
      return Math.min(95, Math.round((distance / power_mw) * 1000));
    }
    return 85;
  }

  function calculateTravelTime() {
    if (distance > 0) {
      const hours = distance / 3.0;
      const h = Math.floor(hours);
      const m = Math.floor((hours - h) * 60);
      return `${h}h ${m}m`;
    }
    return '0h 0m';
  }

  let carControls = { forward: false, backward: false, left: false, right: false, corner_left: false, corner_right: false };
  let isStreaming = true;
  let controlMode = 'open';
  let isManualControlEnabled = true;

  const keyMap = { w: 'forward', s: 'backward', a: 'corner_left', d: 'corner_right' };

  function handleKeyDown(e) {
    const direction = keyMap[e.key.toLowerCase()];
    if (direction && !carControls[direction]) startMovement(direction);
  }

  function handleKeyUp(e) {
    const direction = keyMap[e.key.toLowerCase()];
    if (direction) stopMovement(direction);
  }

  onMount(() => {
    wsClient = initWebSocket('ws://localhost:8081');
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
  });
  onDestroy(() => {
    if (wsClient) wsClient.disconnect();
    window.removeEventListener('keydown', handleKeyDown);
    window.removeEventListener('keyup', handleKeyUp);
  });

  function setControlMode(mode) {
    controlMode = mode;
    isManualControlEnabled = (mode === 'open');
    if (wsClient) wsClient.send({ type: 'control_mode', payload: mode === 'open' ? 0 : 1 });
  }

  function startMovement(direction) {
    if (!isManualControlEnabled) return;
    carControls[direction] = true;
    if (wsClient) wsClient.send({ type: 'command', payload: direction });
    // Update drivestate for trail map integration
    const dsMap = { forward: 1, backward: 2, left: 1, right: 1, corner_left: 1, corner_right: 1 };
    if (dsMap[direction]) telemetryData.update(c => ({ ...c, drivestate: dsMap[direction] }));
  }

  function stopMovement(direction) {
    if (!isManualControlEnabled) return;
    if (!carControls[direction]) return;
    carControls[direction] = false;
    if (wsClient) wsClient.send({ type: 'command', payload: 'stop' });
    telemetryData.update(c => ({ ...c, drivestate: 0 }));
  }
</script>


<div class="min-h-screen bg-gradient-to-br from-[#7a3c1f] via-[#914e24] to-[#c1440e] p-6 font-sans">
  <div class="max-w-7xl mx-auto bg-black rounded-2xl shadow-2xl p-8 border border-gray-700">

    <!-- Header -->
    <div class="text-center mb-8">
      <h1 class="text-4xl font-bold bg-gradient-to-r from-amber-400 to-orange-500 bg-clip-text text-transparent mb-2">
          Lagadan ISDP G5
      </h1>
      <div class="flex justify-center items-center space-x-4 mt-4">
        <div class="flex items-center space-x-2">
          <div class="w-3 h-3 {$connectionStatus === 'connected' ? 'bg-green-500' : 'bg-red-500'} rounded-full animate-pulse"></div>
          <span class="{$connectionStatus === 'connected' ? 'text-green-400' : 'text-red-300'} text-sm">
            WebSocket {$connectionStatus === 'connected' ? 'Connected' : 'Disconnected'}
          </span>
        </div>
        <div class="text-amber-200 text-sm">•</div>
        <div class="w-3 h-3 bg-green-500 rounded-full animate-pulse"></div>
        <div class="text-amber-300 text-sm">Stream: {PI_IP}:8889</div>
      </div>
    </div>

    <!-- Main Grid -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">

      <!-- ══ LEFT COLUMN ══ -->
      <div class="lg:col-span-2 space-y-8">

        <!-- Live Camera -->
        <div class="bg-gradient-to-br from-amber-900/50 to-orange-900/50 rounded-2xl p-6 border border-amber-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-4">
            <span class="bg-gradient-to-r from-amber-400 to-orange-400 bg-clip-text text-transparent">Live feed</span>
          </h2>
          <div class="bg-black rounded-xl overflow-hidden border-2 border-amber-400 shadow-inner relative">
            <iframe src={CAMERA_URL} width="100%" height="480" class="w-full" frameborder="0" allow="camera" title="Live Parking Robot Camera Feed"></iframe>
            {#if !isStreaming}
              <div class="absolute inset-0 flex items-center justify-center bg-gray-900">
                <div class="text-center text-amber-200">
                  <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-amber-400 mx-auto mb-4"></div>
                  <p class="text-lg">Connecting to robot camera...</p>
                  <p class="text-amber-300 text-sm mt-2">{CAMERA_URL}</p>
                </div>
              </div>
            {/if}
            <div class="absolute top-4 right-4 bg-red-600 text-white px-3 py-1 rounded-full text-sm font-bold flex items-center space-x-2">
              <div class="w-2 h-2 bg-white rounded-full animate-pulse"></div>
              <span>LIVE FEED</span>
            </div>
          </div>
        </div>

        <!-- ════ LEFT & RIGHT MOTOR PANELS ════ -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">

          <!-- Left Motor -->
          <div class="bg-gradient-to-br from-blue-900/50 to-cyan-900/50 rounded-2xl p-6 border border-blue-500/30 shadow-lg">
            <h2 class="text-xl font-bold text-white mb-4 flex items-center space-x-2">
              <span class="bg-gradient-to-r from-blue-400 to-cyan-400 bg-clip-text text-transparent">Left Motor (lmtr)</span>
            </h2>
            <div class="space-y-3">
              <div class="flex justify-between items-center py-2 px-3 bg-blue-500/20 rounded-lg border border-blue-400/30">
                <span class="text-blue-200 text-sm font-medium">Power:</span>
                <span class="font-bold text-white">{($telemetryData.lmtr_power_mw || 0).toFixed(1)} mW</span>
              </div>
              <div class="flex justify-between items-center py-2 px-3 bg-blue-500/20 rounded-lg border border-blue-400/30">
                <span class="text-blue-200 text-sm font-medium">Load Current:</span>
                <span class="font-bold text-white">{($telemetryData.lmtr_current_ma || 0).toFixed(1)} mA</span>
              </div>
              <div class="flex justify-between items-center py-2 px-3 bg-blue-500/20 rounded-lg border border-blue-400/30">
                <span class="text-blue-200 text-sm font-medium">Bus Voltage:</span>
                <span class="font-bold text-white">{($telemetryData.lmtr_bus_voltage_mv || 0).toFixed(0)} mV</span>
              </div>
              <div class="flex justify-between items-center py-2 px-3 bg-blue-500/20 rounded-lg border border-blue-400/30">
                <span class="text-blue-200 text-sm font-medium">Shunt Voltage:</span>
                <span class="font-bold text-white">{($telemetryData.lmtr_shunt_voltage_mv || 0).toFixed(2)} mV</span>
              </div>
              <div class="flex justify-between items-center py-2 px-3 bg-green-500/20 rounded-lg border border-green-400/30">
                <span class="text-green-200 text-sm font-medium">Encoder Speed:</span>
                <span class="font-bold text-white">{(($telemetryData.lmtr_actual_speed || 0) * MAX_RPM).toFixed(0)} RPM</span>
              </div>
              <div class="mt-1 p-2 bg-black/30 rounded-lg">
                <p class="text-blue-400 text-xs font-mono text-center">palanuk/ec/lmtr/... + anc/lmtr-actual-speed</p>
              </div>
            </div>
          </div>

          <!-- Right Motor -->
          <div class="bg-gradient-to-br from-purple-900/50 to-pink-900/50 rounded-2xl p-6 border border-purple-500/30 shadow-lg">
            <h2 class="text-xl font-bold text-white mb-4 flex items-center space-x-2">
              <span class="bg-gradient-to-r from-purple-400 to-pink-400 bg-clip-text text-transparent">Right Motor (rmtr)</span>
            </h2>
            <div class="space-y-3">
              <div class="flex justify-between items-center py-2 px-3 bg-purple-500/20 rounded-lg border border-purple-400/30">
                <span class="text-purple-200 text-sm font-medium">Power:</span>
                <span class="font-bold text-white">{($telemetryData.rmtr_power_mw || 0).toFixed(1)} mW</span>
              </div>
              <div class="flex justify-between items-center py-2 px-3 bg-purple-500/20 rounded-lg border border-purple-400/30">
                <span class="text-purple-200 text-sm font-medium">Load Current:</span>
                <span class="font-bold text-white">{($telemetryData.rmtr_current_ma || 0).toFixed(1)} mA</span>
              </div>
              <div class="flex justify-between items-center py-2 px-3 bg-purple-500/20 rounded-lg border border-purple-400/30">
                <span class="text-purple-200 text-sm font-medium">Bus Voltage:</span>
                <span class="font-bold text-white">{($telemetryData.rmtr_bus_voltage_mv || 0).toFixed(0)} mV</span>
              </div>
              <div class="flex justify-between items-center py-2 px-3 bg-purple-500/20 rounded-lg border border-purple-400/30">
                <span class="text-purple-200 text-sm font-medium">Shunt Voltage:</span>
                <span class="font-bold text-white">{($telemetryData.rmtr_shunt_voltage_mv || 0).toFixed(2)} mV</span>
              </div>
              <div class="flex justify-between items-center py-2 px-3 bg-green-500/20 rounded-lg border border-green-400/30">
                <span class="text-green-200 text-sm font-medium">Encoder Speed:</span>
                <span class="font-bold text-white">{(($telemetryData.rmtr_actual_speed || 0) * MAX_RPM).toFixed(0)} RPM</span>
              </div>
              <div class="mt-1 p-2 bg-black/30 rounded-lg">
                <p class="text-purple-400 text-xs font-mono text-center">palanuk/ec/rmtr/... + anc/rmtr-actual-speed</p>
              </div>
            </div>
          </div>

        </div><!-- end motor panels grid -->

        <!-- ════ Combined Motor Totals ════ -->
        <div class="bg-gradient-to-br from-gray-800/80 to-gray-900/80 rounded-2xl p-5 border border-gray-500/40 shadow-lg">
          <h2 class="text-lg font-bold text-white mb-3">
            <span class="bg-gradient-to-r from-yellow-400 to-amber-400 bg-clip-text text-transparent">
              Combined Motor Totals
            </span>
          </h2>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
            <div class="text-center py-3 px-2 bg-yellow-500/10 rounded-lg border border-yellow-400/30">
              <p class="text-yellow-300 text-xs mb-1">Total Power</p>
              <p class="text-white font-bold text-lg">{($motorTotals.total_power_mw || 0).toFixed(1)}</p>
              <p class="text-yellow-400 text-xs">mW</p>
            </div>
            <div class="text-center py-3 px-2 bg-green-500/10 rounded-lg border border-green-400/30">
              <p class="text-green-300 text-xs mb-1">Total Current</p>
              <p class="text-white font-bold text-lg">{($motorTotals.total_current_ma || 0).toFixed(1)}</p>
              <p class="text-green-400 text-xs">mA</p>
            </div>
            <div class="text-center py-3 px-2 bg-cyan-500/10 rounded-lg border border-cyan-400/30">
              <p class="text-cyan-300 text-xs mb-1">Avg Bus Voltage</p>
              <p class="text-white font-bold text-lg">{($motorTotals.avg_bus_voltage_mv || 0).toFixed(0)}</p>
              <p class="text-cyan-400 text-xs">mV</p>
            </div>
            <div class="text-center py-3 px-2 bg-pink-500/10 rounded-lg border border-pink-400/30">
              <p class="text-pink-300 text-xs mb-1">Avg Shunt Voltage</p>
              <p class="text-white font-bold text-lg">{($motorTotals.avg_shunt_voltage_mv || 0).toFixed(2)}</p>
              <p class="text-pink-400 text-xs">mV</p>
            </div>
          </div>
        </div>

        <!-- Message Log -->
        <!-- <div class="bg-gradient-to-br from-red-900/50 to-amber-900/50 rounded-2xl p-6 border border-red-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-4">
            <span class="bg-gradient-to-r from-red-400 to-amber-400 bg-clip-text text-transparent">Message log</span>
          </h2>
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-xl p-4 border border-red-400/30">
            <div class="max-h-64 overflow-y-auto space-y-2">
              {#each $messageLog.slice(-10).reverse() as msg}
                <div class="bg-gradient-to-r from-red-500/10 to-amber-500/10 rounded-lg p-3 border border-red-400/20">
                  <div class="flex items-start justify-between">
                    <span class="text-red-300 text-xs font-mono">{msg.time}</span>
                    <span class="text-amber-300 text-xs">{msg.data.type || 'unknown'}</span>
                  </div>
                  <pre class="text-amber-100 text-xs mt-2 overflow-x-auto">{JSON.stringify(msg.data, null, 2)}</pre>
                </div>
              {:else}
                <p class="text-red-300 text-center py-4">No messages yet...</p>
              {/each}
            </div>
          </div>
        </div> -->
      </div><!-- end left column -->


      <!-- ══ RIGHT COLUMN ══ -->
      <div class="space-y-8">

        <!-- Robot Energy Data (5V system) -->
        <div class="bg-gradient-to-br from-red-900/50 to-amber-900/50 rounded-2xl p-6 border border-red-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-1">
            <span class="bg-gradient-to-r from-red-400 to-amber-400 bg-clip-text text-transparent">Power system</span>
          </h2>
          <p class="text-gray-400 text-xs mb-4">5V system — ITP main board</p>
          <div class="space-y-4">
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-red-500/20 to-pink-500/20 rounded-lg border border-red-400/30">
              <span class="text-red-200 font-medium">Power Consumption:</span>
              <span class="font-bold text-white text-lg">{robotData.power} mW</span>
            </div>
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-orange-500/20 to-amber-500/20 rounded-lg border border-orange-400/30">
              <span class="text-orange-200 font-medium">Current Draw:</span>
              <span class="font-bold text-white text-lg">{robotData.current} mA</span>
            </div>
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-amber-500/20 to-yellow-500/20 rounded-lg border border-amber-400/30">
              <span class="text-amber-200 font-medium">Bus Voltage:</span>
              <span class="font-bold text-white text-lg">{robotData.voltage} mV</span>
            </div>
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-yellow-500/20 to-amber-500/20 rounded-lg border border-yellow-400/30">
              <span class="text-yellow-200 font-medium">Shunt Voltage:</span>
              <span class="font-bold text-white text-lg">{robotData.shunt} mV</span>
            </div>
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-red-500/20 to-pink-500/20 rounded-lg border border-red-400/30">
              <span class="text-red-200 font-medium">Distance:</span>
              <span class="font-bold text-white text-lg">{robotData.distance} cm</span>
            </div>
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-orange-500/20 to-red-500/20 rounded-lg border border-orange-400/30">
              <span class="text-orange-200 font-medium">Power (W):</span>
              <span class="font-bold text-white text-lg">{robotData.totalEnergy} W</span>
            </div>
          </div>
        </div>

        <!-- Control Mode -->
        <div class="bg-gradient-to-br from-orange-900/50 to-red-900/50 rounded-2xl p-6 border border-orange-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-4">
            <span class="bg-gradient-to-r from-orange-400 to-red-400 bg-clip-text text-transparent">Line following</span>
          </h2>
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-xl p-4 border border-orange-400/30">
            <div class="grid grid-cols-2 gap-3">
              <button on:click={() => setControlMode('open')}
                class="py-3 px-3 rounded-lg font-bold text-sm transition-all duration-300 transform hover:scale-105 {controlMode === 'open' ? 'bg-gradient-to-r from-amber-500 to-orange-600 text-white shadow-lg shadow-amber-500/50' : 'bg-gray-700 text-gray-400 hover:bg-gray-600'}">
                <div class="flex flex-col items-center space-y-1">
                  <span class="text-xs">Open Loop</span>
                  <span class="text-xs opacity-75">(Manual)</span>
                  {#if controlMode === 'open'}<span class="text-sm">✓</span>{/if}
                </div>
              </button>
              <button on:click={() => setControlMode('closed')}
                class="py-3 px-3 rounded-lg font-bold text-sm transition-all duration-300 transform hover:scale-105 {controlMode === 'closed' ? 'bg-gradient-to-r from-red-500 to-pink-600 text-white shadow-lg shadow-red-500/50' : 'bg-gray-700 text-gray-400 hover:bg-gray-600'}">
                <div class="flex flex-col items-center space-y-1">
                  <span class="text-xs">Closed Loop</span>
                  <span class="text-xs opacity-75">(Auto)</span>
                  {#if controlMode === 'closed'}<span class="text-sm">✓</span>{/if}
                </div>
              </button>
            </div>
            <div class="mt-3 p-2 bg-black/30 rounded-lg">
              <p class="text-xs text-center">
                {#if controlMode === 'open'}
                  <span class="text-amber-400">✓ Manual control enabled</span>
                {:else}
                  <span class="text-red-400">✓ Autonomous mode active</span>
                {/if}
              </p>
            </div>
          </div>
        </div>

        <!-- D-Pad Controls -->
        <div class="bg-gradient-to-br from-red-900/50 to-pink-900/50 rounded-2xl p-6 border border-red-500/30 shadow-lg {!isManualControlEnabled ? 'opacity-50' : ''}">
          <h2 class="text-2xl font-bold text-white mb-4 flex items-center">
            <span class="bg-gradient-to-r from-red-400 to-pink-400 bg-clip-text text-transparent">Robot Controls</span>
            {#if !isManualControlEnabled}
              <span class="ml-2 text-xs bg-red-500 text-white px-2 py-1 rounded">AUTO</span>
            {/if}
          </h2>
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-xl p-6 border border-red-400/30 relative">
            {#if !isManualControlEnabled}
              <div class="absolute inset-0 bg-black/60 rounded-xl flex items-center justify-center z-10">
                <div class="text-center px-4">
                  <p class="text-white text-base font-bold mb-2">Autonomous Mode</p>
                  <p class="text-gray-300 text-xs">Switch to Open Loop for manual control</p>
                </div>
              </div>
            {/if}
            <div class="flex flex-col items-center space-y-4">
              <button on:mousedown={() => startMovement('forward')} on:mouseup={() => stopMovement('forward')} on:mouseleave={() => stopMovement('forward')}
                disabled={!isManualControlEnabled}
                class="w-20 h-20 bg-gradient-to-br from-amber-500 to-orange-600 hover:from-amber-600 hover:to-orange-700 text-white rounded-xl flex items-center justify-center text-3xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 {carControls.forward ? 'scale-105 shadow-2xl' : ''}">↑</button>
              <div class="flex space-x-8">
                <button on:mousedown={() => startMovement('corner_left')} on:mouseup={() => stopMovement('corner_left')} on:mouseleave={() => stopMovement('corner_left')}
                  disabled={!isManualControlEnabled}
                  class="w-20 h-20 bg-gradient-to-br from-amber-600 to-yellow-700 hover:from-amber-700 hover:to-yellow-800 text-white rounded-xl flex items-center justify-center text-2xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 {carControls.corner_left ? 'scale-105 shadow-2xl' : ''}">↺</button>
                <button on:mousedown={() => startMovement('corner_right')} on:mouseup={() => stopMovement('corner_right')} on:mouseleave={() => stopMovement('corner_right')}
                  disabled={!isManualControlEnabled}
                  class="w-20 h-20 bg-gradient-to-br from-amber-600 to-yellow-700 hover:from-amber-700 hover:to-yellow-800 text-white rounded-xl flex items-center justify-center text-2xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 {carControls.corner_right ? 'scale-105 shadow-2xl' : ''}">↻</button>
              </div>
              <button on:mousedown={() => startMovement('backward')} on:mouseup={() => stopMovement('backward')} on:mouseleave={() => stopMovement('backward')}
                disabled={!isManualControlEnabled}
                class="w-20 h-20 bg-gradient-to-br from-red-600 to-pink-700 hover:from-red-700 hover:to-pink-800 text-white rounded-xl flex items-center justify-center text-3xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 {carControls.backward ? 'scale-105 shadow-2xl' : ''}">↓</button>
              <div class="flex space-x-4 mt-4 items-center">
                <button on:mousedown={() => startMovement('left')} on:mouseup={() => stopMovement('left')} on:mouseleave={() => stopMovement('left')}
                  disabled={!isManualControlEnabled}
                  class="w-20 h-20 bg-gradient-to-br from-red-500 to-pink-600 hover:from-red-600 hover:to-pink-700 text-white rounded-xl flex items-center justify-center text-xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 {carControls.left ? 'scale-105 shadow-2xl' : ''}">📷←</button>
                <button on:click={() => { if (wsClient) wsClient.send({ type: 'command', payload: 'pan_center' }); }}
                  disabled={!isManualControlEnabled}
                  class="w-20 h-20 bg-gradient-to-br from-red-500 to-pink-600 hover:from-red-600 hover:to-pink-700 text-white rounded-xl flex items-center justify-center text-xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100">📷</button>
                <button on:mousedown={() => startMovement('right')} on:mouseup={() => stopMovement('right')} on:mouseleave={() => stopMovement('right')}
                  disabled={!isManualControlEnabled}
                  class="w-20 h-20 bg-gradient-to-br from-red-500 to-pink-600 hover:from-red-600 hover:to-pink-700 text-white rounded-xl flex items-center justify-center text-xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 {carControls.right ? 'scale-105 shadow-2xl' : ''}">→📷</button>
              </div>
            </div>
            <div class="mt-6 text-center">
              <p class="text-red-200 font-medium">{isManualControlEnabled ? 'Manual Control' : 'Autonomous Mode'}</p>
              <p class="text-red-300 text-xs mt-2">{isManualControlEnabled ? 'Press and hold for movement' : 'Vehicle running autonomously'}</p>
            </div>
          </div>
        </div>

        <!-- Command Feedback -->
        <div class="bg-gradient-to-br from-gray-800/80 to-gray-900/80 rounded-2xl p-4 border border-gray-600/30 shadow-lg">
          <h2 class="text-sm font-bold text-gray-400 mb-2">Command Feedback</h2>
          <div class="max-h-48 overflow-y-auto space-y-1">
            {#each $commandFeedback.slice(-10).reverse() as fb}
              <div class="flex items-center space-x-2 text-sm py-1 px-2 bg-black/20 rounded">
                <span>{fb.success ? '' : '❌'}</span>
                <span class="text-amber-200 truncate">{fb.message}</span>
                <span class="text-gray-500 text-xs ml-auto whitespace-nowrap">{fb.timestamp}</span>
              </div>
            {:else}
              <p class="text-gray-600 text-xs">No recent commands</p>
            {/each}
          </div>
        </div>

      </div><!-- end right column -->
    </div><!-- end main grid -->

    <!-- Footer -->
    <div class="mt-8 text-center">
      <p class="text-gray-400 text-sm"><span class="text-amber-400">{new Date().toLocaleString()}</span></p>
    </div>

  </div>
</div>
