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
  } from '$lib/stores/zenohStore';
 
  // Pi's IP address
  const PI_IP = '169.254.219.175';
  const CAMERA_URL = `http://${PI_IP}:8889/cam/`;
 
  let wsClient;

  // Update the reactive robotData to use real telemetry
  $: robotData = {
    totalEnergy: ((Number($telemetryData?.power_w) || 0) * 0.001).toFixed(2), // Convert W to kW
    efficiency: calculateEfficiency(),
    averageSpeed: Math.round(($vehicleState?.status?.includes('moving'))? 10.8:0),
    travelTime: calculateTravelTime(),
    distance: (Number($telemetryData?.distance) || 0).toFixed(2),
    battery: Math.round($telemetryData?.battery) || 0,
    power: ($telemetryData.power_w * 1000).toFixed(1), // Convert W back to mW for display
  current: ($telemetryData.current_a * 1000).toFixed(1), // Convert A back to mA
  voltage: ($telemetryData.voltage_v * 1000).toFixed(0) // Convert V back to mV
  };

  function calculateEfficiency() {
    // Calculate efficiency based on power usage vs distance
    // This is a placeholder - adjust based on your metrics
    if ($telemetryData.distance > 0 && $telemetryData.power_w > 0) {
      return Math.min(95, Math.round(($telemetryData.distance / $telemetryData.power_w) * 100));
    }
    return 85; // Default
  }

  function calculateTravelTime() {
    // Calculate based on distance and average speed
    if ($telemetryData.distance > 0) {
      const hours = $telemetryData.distance / 3.0; // Assuming 3 m/s average
      const h = Math.floor(hours);
      const m = Math.floor((hours - h) * 60);
      return `${h}h ${m}m`;
    }
    return '0h 0m';
  }
 
  let recordedImages = [
    { id: 1, name: 'parking_001.jpg', type: 'obstacle' },
    { id: 2, name: 'path_clear.jpg', type: 'path' },
    { id: 3, name: 'terrain_scan.jpg', type: 'terrain' },
    { id: 4, name: 'navigation.jpg', type: 'navigation' }
  ];
 
  let carControls = {
    forward: false,
    backward: false,
    left: false,
    right: false
  };
 
  let isStreaming = true;

  // NEW: Control mode state
  let controlMode = 'open';  // 'open' = manual, 'closed' = autonomous
  let isManualControlEnabled = true;

  onMount(() => {
    wsClient = initWebSocket('ws://localhost:8081');
  });

  onDestroy(() => {
    if (wsClient) {
      wsClient.disconnect();
    }
  });

  // NEW: Function to change control mode
  function setControlMode(mode) {
    controlMode = mode;
    isManualControlEnabled = (mode === 'open');
    
    if (wsClient) {
      wsClient.send({
        type: 'control_mode',
        payload: mode === 'open' ? 0 : 1  // 0 = open loop, 1 = closed loop
      });
    }
    
    console.log(`Control mode set to: ${mode} loop`);
  }

  function startMovement(direction) {
    // Only allow manual control in open loop mode
    if (!isManualControlEnabled) {
      console.warn('Manual control disabled in closed loop mode');
      return;
    }
    
    carControls[direction] = true;
    console.log(`Moving ${direction}`);
    
    if (wsClient) {
      wsClient.send({
        type: 'command',
        payload: direction
      });
    }
  }
 
  function stopMovement(direction) {
    if (!isManualControlEnabled) return;
    
    carControls[direction] = false;
    console.log(`Stopped ${direction}`);
    
    if (wsClient) {
      wsClient.send({
        type: 'command',
        payload: 'stop'
      });
    }
  }
 
  function getImageColor(type) {
    const colors = {
      obstacle: 'from-red-500 to-pink-600',
      path: 'from-green-500 to-emerald-600',
      terrain: 'from-yellow-500 to-orange-600',
      navigation: 'from-blue-500 to-cyan-600'
    };
    return colors[type] || 'from-gray-500 to-gray-600';
  }
</script>


<div class="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-800 p-6 font-sans">
  <!-- Main Container -->
  <div class="max-w-7xl mx-auto bg-gradient-to-br from-gray-800 to-gray-900 rounded-2xl shadow-2xl p-8 border border-gray-700">
   
    <!-- Header -->
    <div class="text-center mb-8">
      <h1 class="text-4xl font-bold bg-gradient-to-r from-cyan-400 to-blue-500 bg-clip-text text-transparent mb-2">
        Parking Robot Dashboard
      </h1>
      <div class="flex justify-center items-center space-x-4 mt-4">
        <div class="flex items-center space-x-2">
          <div class="w-3 h-3 {$connectionStatus === 'connected' ? 'bg-green-500' : 'bg-red-500'} rounded-full animate-pulse"></div>
          <span class="{$connectionStatus === 'connected' ? 'text-green-400' : 'text-red-400'} text-sm">
            WebSocket {$connectionStatus === 'connected' ? 'Connected' : 'Disconnected'}
          </span>
        </div>
        <div class="text-gray-400 text-sm">•</div>
        <div class="flex items-center space-x-2">
          <div class="w-3 h-3 bg-green-500 rounded-full animate-pulse"></div>
          <span class="text-green-400 text-sm">Camera Online</span>
        </div>
        <div class="text-gray-400 text-sm">•</div>
        <div class="text-cyan-400 text-sm">Stream: {PI_IP}:8889</div>
      </div>
    </div>

    <!-- Command Feedback -->
    {#if $commandFeedback}
      <div class="mb-6 bg-gradient-to-r from-green-500/20 to-emerald-500/20 border border-green-400/30 rounded-xl p-4 animate-pulse">
        <div class="flex items-center justify-center space-x-3">
          <span class="text-2xl">{$commandFeedback.success ? '✅' : '❌'}</span>
          <span class="text-green-200 font-bold">{$commandFeedback.message}</span>
          <span class="text-green-300 text-sm">{$commandFeedback.timestamp}</span>
        </div>
      </div>
    {/if}


    <!-- Main Content Grid -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
     
      <!-- Left Column -->
      <div class="lg:col-span-2 space-y-8">
       
        <!-- REAL Pi Camera Feed Section -->
        <div class="bg-gradient-to-br from-emerald-900/50 to-green-900/50 rounded-2xl p-6 border border-emerald-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-4 flex items-center">
            <span class="mr-3 text-3xl">📹</span>
            <span class="bg-gradient-to-r from-emerald-400 to-green-400 bg-clip-text text-transparent">
              Live Parking Camera
            </span>
          </h2>
          <div class="bg-black rounded-xl overflow-hidden border-2 border-emerald-400 shadow-inner relative">
            <iframe
              src={CAMERA_URL}
              width="100%"
              height="480"
              class="w-full"
              frameborder="0"
              allow="camera"
              title="Live Parking Robot Camera Feed"
            ></iframe>
           
            {#if !isStreaming}
              <div class="absolute inset-0 flex items-center justify-center bg-gray-900">
                <div class="text-center text-emerald-200">
                  <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-emerald-400 mx-auto mb-4"></div>
                  <p class="text-lg">Connecting to robot camera...</p>
                  <p class="text-emerald-300 text-sm mt-2">{CAMERA_URL}</p>
                </div>
              </div>
            {/if}
           
            <!-- Live Badge -->
            <div class="absolute top-4 right-4 bg-red-600 text-white px-3 py-1 rounded-full text-sm font-bold flex items-center space-x-2">
              <div class="w-2 h-2 bg-white rounded-full animate-pulse"></div>
              <span>LIVE FEED</span>
            </div>
          </div>
        </div>


        

        <!-- Message Log Section -->
        <div class="bg-gradient-to-br from-indigo-900/50 to-purple-900/50 rounded-2xl p-6 border border-indigo-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-4 flex items-center">
            <span class="mr-3 text-3xl">📜</span>
            <span class="bg-gradient-to-r from-indigo-400 to-purple-400 bg-clip-text text-transparent">
              Message Log
            </span>
          </h2>
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-xl p-4 border border-indigo-400/30">
            <div class="max-h-64 overflow-y-auto space-y-2">
              {#each $messageLog.slice(-10).reverse() as msg}
                <div class="bg-gradient-to-r from-indigo-500/10 to-purple-500/10 rounded-lg p-3 border border-indigo-400/20">
                  <div class="flex items-start justify-between">
                    <span class="text-indigo-300 text-xs font-mono">{msg.time}</span>
                    <span class="text-purple-300 text-xs">{msg.data.type || 'unknown'}</span>
                  </div>
                  <pre class="text-indigo-100 text-xs mt-2 overflow-x-auto">{JSON.stringify(msg.data, null, 2)}</pre>
                </div>
              {:else}
                <p class="text-indigo-300 text-center py-4">No messages yet...</p>
              {/each}
            </div>
          </div>
        </div>

      </div>

      <!-- Right Column -->
       <div class="space-y-8">

        
      <div class="bg-gradient-to-br from-amber-900/50 to-orange-900/50 rounded-2xl p-6 border border-amber-500/30 shadow-lg">
        <h2 class="text-2xl font-bold text-white mb-4 flex items-center">
          <span class="mr-3 text-3xl">📊</span>
          <span class="bg-gradient-to-r from-amber-400 to-orange-400 bg-clip-text text-transparent">
            Robot Energy Data
          </span>
        </h2>
        <div class="space-y-4">
          <!-- Battery Level -->
          <div class="py-3 px-4 bg-gradient-to-r from-indigo-500/20 to-purple-500/20 rounded-lg border border-indigo-400/30">
            <div class="flex justify-between items-center mb-1">
              <span class="text-indigo-200 font-medium">Battery Level:</span>
              <span class="font-bold text-white text-lg">{robotData.battery}%</span>
            </div>
            <div class="w-full bg-gray-700 rounded-full h-2">
              <div 
                class="bg-gradient-to-r from-green-400 to-cyan-400 h-2 rounded-full transition-all duration-500"
                style="width: {robotData.battery}%;"
              ></div>
            </div>
          </div>
          
          <!-- Power Consumption -->
          <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-red-500/20 to-pink-500/20 rounded-lg border border-red-400/30">
            <span class="text-red-200 font-medium">Power Consumption:</span>
            <span class="font-bold text-white text-lg">{robotData.power} mW</span>
          </div>
          
          <!-- Current Draw -->
          <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-blue-500/20 to-cyan-500/20 rounded-lg border border-blue-400/30">
            <span class="text-blue-200 font-medium">Current Draw:</span>
            <span class="font-bold text-white text-lg">{robotData.current} mA</span>
          </div>
          
          <!-- System Voltage -->
          <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-green-500/20 to-emerald-500/20 rounded-lg border border-green-400/30">
            <span class="text-green-200 font-medium">System Voltage:</span>
            <span class="font-bold text-white text-lg">{robotData.voltage} mV</span>
          </div>

          <!-- Distance -->
      <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-purple-500/20 to-pink-500/20 rounded-lg border border-purple-400/30">
        <span class="text-purple-200 font-medium">Distance:</span>
        <span class="font-bold text-white text-lg">{robotData.distance} cm</span>
      </div>
          
          <!-- Energy Used -->
          <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-amber-500/20 to-yellow-500/20 rounded-lg border border-amber-400/30">
            <span class="text-amber-200 font-medium">Energy Used:</span>
            <span class="font-bold text-white text-lg">{robotData.totalEnergy} mW</span>
          </div>
          
          <!-- Efficiency -->
          <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-purple-500/20 to-pink-500/20 rounded-lg border border-purple-400/30">
            <span class="text-purple-200 font-medium">Power Efficiency:</span>
            <span class="font-bold text-white text-lg">{robotData.efficiency}%</span>
          </div>
        </div>
      </div>


        <!-- NEW: Control Mode Selector -->
        <div class="bg-gradient-to-br from-purple-900/50 to-indigo-900/50 rounded-2xl p-6 border border-purple-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-4 flex items-center">
            <span class="mr-3 text-3xl">🎛️</span>
            <span class="bg-gradient-to-r from-purple-400 to-indigo-400 bg-clip-text text-transparent">
              Control Mode
            </span>
          </h2>
          
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-xl p-4 border border-purple-400/30">
            <div class="grid grid-cols-2 gap-3">
              <!-- Open Loop Button -->
              <button
                on:click={() => setControlMode('open')}
                class="py-3 px-3 rounded-lg font-bold text-sm transition-all duration-300 transform hover:scale-105 {
                  controlMode === 'open' 
                    ? 'bg-gradient-to-r from-green-500 to-emerald-600 text-white shadow-lg shadow-green-500/50' 
                    : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
                }"
              >
                <div class="flex flex-col items-center space-y-1">
                  <span class="text-xl">🎮</span>
                  <span class="text-xs">Open Loop</span>
                  <span class="text-xs opacity-75">(Manual)</span>
                  {#if controlMode === 'open'}
                    <span class="text-sm">✓</span>
                  {/if}
                </div>
              </button>
              
              <!-- Closed Loop Button -->
              <button
                on:click={() => setControlMode('closed')}
                class="py-3 px-3 rounded-lg font-bold text-sm transition-all duration-300 transform hover:scale-105 {
                  controlMode === 'closed' 
                    ? 'bg-gradient-to-r from-blue-500 to-cyan-600 text-white shadow-lg shadow-blue-500/50' 
                    : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
                }"
              >
                <div class="flex flex-col items-center space-y-1">
                  <span class="text-xl">🤖</span>
                  <span class="text-xs">Closed Loop</span>
                  <span class="text-xs opacity-75">(Auto)</span>
                  {#if controlMode === 'closed'}
                    <span class="text-sm">✓</span>
                  {/if}
                </div>
              </button>
            </div>
            
            <!-- Status Indicator -->
            <div class="mt-3 p-2 bg-black/30 rounded-lg">
              <p class="text-xs text-center">
                {#if controlMode === 'open'}
                  <span class="text-green-400">✓ Manual control enabled</span>
                {:else}
                  <span class="text-blue-400">✓ Autonomous mode active</span>
                {/if}
              </p>
            </div>
          </div>
        </div>


        <!-- Car Controls Section -->
        <div class="bg-gradient-to-br from-red-900/50 to-pink-900/50 rounded-2xl p-6 border border-red-500/30 shadow-lg {!isManualControlEnabled ? 'opacity-50' : ''}">
          <h2 class="text-2xl font-bold text-white mb-4 flex items-center">
            <span class="mr-3 text-3xl">🎮</span>
            <span class="bg-gradient-to-r from-red-400 to-pink-400 bg-clip-text text-transparent">
              Robot Controls
            </span>
            {#if !isManualControlEnabled}
              <span class="ml-2 text-xs bg-blue-500 text-white px-2 py-1 rounded">AUTO</span>
            {/if}
          </h2>
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-xl p-6 border border-red-400/30 relative">
            
            <!-- Disabled Overlay -->
            {#if !isManualControlEnabled}
              <div class="absolute inset-0 bg-black/60 rounded-xl flex items-center justify-center z-10">
                <div class="text-center px-4">
                  <p class="text-white text-base font-bold mb-2">🤖 Autonomous Mode</p>
                  <p class="text-gray-300 text-xs">Switch to Open Loop for manual control</p>
                </div>
              </div>
            {/if}
            
            <div class="flex flex-col items-center space-y-4">
              <!-- Forward Button -->
              <button
                on:mousedown={() => startMovement('forward')}
                on:mouseup={() => stopMovement('forward')}
                on:mouseleave={() => stopMovement('forward')}
                disabled={!isManualControlEnabled}
                class="w-20 h-20 bg-gradient-to-br from-green-500 to-emerald-600 hover:from-green-600 hover:to-emerald-700 text-white rounded-xl flex items-center justify-center text-3xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 {carControls.forward ? 'from-green-600 to-emerald-700 scale-105 shadow-2xl' : ''}"
              >
                ↑
              </button>
             
              <!-- Middle Row - Left and Right -->
              <div class="flex space-x-8">
                <button
                  on:mousedown={() => startMovement('left')}
                  on:mouseup={() => stopMovement('left')}
                  on:mouseleave={() => stopMovement('left')}
                  disabled={!isManualControlEnabled}
                  class="w-20 h-20 bg-gradient-to-br from-blue-500 to-cyan-600 hover:from-blue-600 hover:to-cyan-700 text-white rounded-xl flex items-center justify-center text-3xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 {carControls.left ? 'from-blue-600 to-cyan-700 scale-105 shadow-2xl' : ''}"
                >
                  ←
                </button>
                <button
                  on:mousedown={() => startMovement('right')}
                  on:mouseup={() => stopMovement('right')}
                  on:mouseleave={() => stopMovement('right')}
                  disabled={!isManualControlEnabled}
                  class="w-20 h-20 bg-gradient-to-br from-blue-500 to-cyan-600 hover:from-blue-600 hover:to-cyan-700 text-white rounded-xl flex items-center justify-center text-3xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 {carControls.right ? 'from-blue-600 to-cyan-700 scale-105 shadow-2xl' : ''}"
                >
                  →
                </button>
              </div>
             
              <!-- Backward Button -->
              <button
                on:mousedown={() => startMovement('backward')}
                on:mouseup={() => stopMovement('backward')}
                on:mouseleave={() => stopMovement('backward')}
                disabled={!isManualControlEnabled}
                class="w-20 h-20 bg-gradient-to-br from-orange-500 to-red-600 hover:from-orange-600 hover:to-red-700 text-white rounded-xl flex items-center justify-center text-3xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 {carControls.backward ? 'from-orange-600 to-red-700 scale-105 shadow-2xl' : ''}"
              >
                ↓
              </button>
            </div>
           
            <!-- Control Labels -->
            <div class="mt-6 text-center">
              <p class="text-red-200 font-medium">
                {isManualControlEnabled ? 'Manual Control' : 'Autonomous Mode'}
              </p>
              <p class="text-red-300 text-xs mt-2">
                {isManualControlEnabled ? 'Press and hold for movement' : 'Vehicle running autonomously'}
              </p>
            </div>
          </div>
        </div>


      </div>
    </div>


    <!-- Footer -->
    <div class="mt-8 text-center">
      <p class="text-gray-400 text-sm">
        <span class="text-cyan-400">{new Date().toLocaleString()}</span>
      </p>
    </div>
  </div>
</div>