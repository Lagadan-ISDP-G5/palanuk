<script>
  import { onMount } from 'svelte';
 
  // Pi's IP address
  const PI_IP = '169.254.219.175';
  const CAMERA_URL = `http://${PI_IP}:8889/cam/`;
 
  // Mock data
  let robotData = {
    totalEnergy: 120,
    efficiency: 85,
    averageSpeed: 65,
    distance: 120,
    travelTime: '2h 15m',
    battery: 78
  };
 
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


  function startMovement(direction) {
    carControls[direction] = true;
    console.log(`Moving ${direction}`);
  }
 
  function stopMovement(direction) {
    carControls[direction] = false;
    console.log(`Stopped ${direction}`);
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
        🚗 Parking Robot Dashboard
      </h1>
      <p class="text-gray-300">Real-time monitoring and control interface</p>
      <div class="flex justify-center items-center space-x-4 mt-4">
        <div class="flex items-center space-x-2">
          <div class="w-3 h-3 bg-green-500 rounded-full animate-pulse"></div>
          <span class="text-green-400 text-sm">Camera Online</span>
        </div>
        <div class="text-gray-400 text-sm">•</div>
        <div class="text-cyan-400 text-sm">Stream: {PI_IP}:8889</div>
      </div>
    </div>


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


        <!-- Recorded Images Section -->
        <div class="bg-gradient-to-br from-purple-900/50 to-pink-900/50 rounded-2xl p-6 border border-purple-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-4 flex items-center">
            <span class="mr-3 text-3xl">🖼️</span>
            <span class="bg-gradient-to-r from-purple-400 to-pink-400 bg-clip-text text-transparent">
              Captured Images
            </span>
          </h2>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
            {#each recordedImages as image}
              <div class="bg-gradient-to-br {getImageColor(image.type)} rounded-xl aspect-square flex flex-col items-center justify-center border-2 border-white/20 shadow-lg transform hover:scale-105 transition-transform duration-300 cursor-pointer">
                <div class="text-4xl mb-2 text-white">
                  {#if image.type === 'obstacle'}🚗
                  {:else if image.type === 'path'}🛣️
                  {:else if image.type === 'terrain'}🏔️
                  {:else if image.type === 'navigation'}🧭
                  {:else}📷{/if}
                </div>
                <div class="text-white text-xs text-center font-medium bg-black/30 rounded px-2 py-1">
                  {image.name}
                </div>
              </div>
            {/each}
          </div>
        </div>


      </div>


      <!-- Right Column -->
      <div class="space-y-8">
       
        <!-- Recorded Data Section -->
        <div class="bg-gradient-to-br from-amber-900/50 to-orange-900/50 rounded-2xl p-6 border border-amber-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-4 flex items-center">
            <span class="mr-3 text-3xl">📊</span>
            <span class="bg-gradient-to-r from-amber-400 to-orange-400 bg-clip-text text-transparent">
              Robot Data
            </span>
          </h2>
          <div class="space-y-4">
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-amber-500/20 to-orange-500/20 rounded-lg border border-amber-400/30">
              <span class="text-amber-200 font-medium">Total Energy:</span>
              <span class="font-bold text-white text-lg">{robotData.totalEnergy} kWh</span>
            </div>
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-green-500/20 to-emerald-500/20 rounded-lg border border-green-400/30">
              <span class="text-green-200 font-medium">Efficiency:</span>
              <span class="font-bold text-white text-lg">{robotData.efficiency}%</span>
            </div>
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-blue-500/20 to-cyan-500/20 rounded-lg border border-blue-400/30">
              <span class="text-blue-200 font-medium">Speed:</span>
              <span class="font-bold text-white text-lg">{robotData.averageSpeed} km/h</span>
            </div>
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-purple-500/20 to-pink-500/20 rounded-lg border border-purple-400/30">
              <span class="text-purple-200 font-medium">Distance:</span>
              <span class="font-bold text-white text-lg">{robotData.distance} km</span>
            </div>
            <div class="flex justify-between items-center py-3 px-4 bg-gradient-to-r from-indigo-500/20 to-purple-500/20 rounded-lg border border-indigo-400/30">
              <span class="text-indigo-200 font-medium">Battery:</span>
              <span class="font-bold text-white text-lg">{robotData.battery}%</span>
            </div>
          </div>
        </div>


        <!-- Performance Evaluation Section -->
        <div class="bg-gradient-to-br from-teal-900/50 to-cyan-900/50 rounded-2xl p-6 border border-teal-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-4 flex items-center">
            <span class="mr-3 text-3xl">📈</span>
            <span class="bg-gradient-to-r from-teal-400 to-cyan-400 bg-clip-text text-transparent">
              Performance
            </span>
          </h2>
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-xl p-4 border border-teal-400/30">
            <p class="text-teal-200 text-center text-sm leading-relaxed">
              Parking efficiency, obstacle detection accuracy, and navigation performance metrics.
            </p>
            <div class="mt-4 flex justify-center space-x-2">
              <div class="w-3 h-3 bg-teal-400 rounded-full animate-pulse"></div>
              <div class="w-3 h-3 bg-cyan-400 rounded-full animate-pulse" style="animation-delay: 0.2s"></div>
              <div class="w-3 h-3 bg-teal-400 rounded-full animate-pulse" style="animation-delay: 0.4s"></div>
            </div>
          </div>
        </div>


        <!-- Car Controls Section -->
        <div class="bg-gradient-to-br from-red-900/50 to-pink-900/50 rounded-2xl p-6 border border-red-500/30 shadow-lg">
          <h2 class="text-2xl font-bold text-white mb-4 flex items-center">
            <span class="mr-3 text-3xl">🎮</span>
            <span class="bg-gradient-to-r from-red-400 to-pink-400 bg-clip-text text-transparent">
              Robot Controls
            </span>
          </h2>
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-xl p-6 border border-red-400/30">
            <div class="flex flex-col items-center space-y-4">
              <!-- Forward Button -->
              <button
                on:mousedown={() => startMovement('forward')}
                on:mouseup={() => stopMovement('forward')}
                on:mouseleave={() => stopMovement('forward')}
                class="w-20 h-20 bg-gradient-to-br from-green-500 to-emerald-600 hover:from-green-600 hover:to-emerald-700 text-white rounded-xl flex items-center justify-center text-3xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 {carControls.forward ? 'from-green-600 to-emerald-700 scale-105 shadow-2xl' : ''}"
              >
                ↑
              </button>
             
              <!-- Middle Row - Left and Right -->
              <div class="flex space-x-8">
                <button
                  on:mousedown={() => startMovement('left')}
                  on:mouseup={() => stopMovement('left')}
                  on:mouseleave={() => stopMovement('left')}
                  class="w-20 h-20 bg-gradient-to-br from-blue-500 to-cyan-600 hover:from-blue-600 hover:to-cyan-700 text-white rounded-xl flex items-center justify-center text-3xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 {carControls.left ? 'from-blue-600 to-cyan-700 scale-105 shadow-2xl' : ''}"
                >
                  ←
                </button>
                <button
                  on:mousedown={() => startMovement('right')}
                  on:mouseup={() => stopMovement('right')}
                  on:mouseleave={() => stopMovement('right')}
                  class="w-20 h-20 bg-gradient-to-br from-blue-500 to-cyan-600 hover:from-blue-600 hover:to-cyan-700 text-white rounded-xl flex items-center justify-center text-3xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 {carControls.right ? 'from-blue-600 to-cyan-700 scale-105 shadow-2xl' : ''}"
                >
                  →
                </button>
              </div>
             
              <!-- Backward Button -->
              <button
                on:mousedown={() => startMovement('backward')}
                on:mouseup={() => stopMovement('backward')}
                on:mouseleave={() => stopMovement('backward')}
                class="w-20 h-20 bg-gradient-to-br from-orange-500 to-red-600 hover:from-orange-600 hover:to-red-700 text-white rounded-xl flex items-center justify-center text-3xl font-bold shadow-lg transform hover:scale-110 transition-all duration-200 {carControls.backward ? 'from-orange-600 to-red-700 scale-105 shadow-2xl' : ''}"
              >
                ↓
              </button>
            </div>
           
            <!-- Control Labels -->
            <div class="mt-6 text-center">
              <p class="text-red-200 font-medium">Parking Robot Controls</p>
              <p class="text-red-300 text-xs mt-2">Press and hold for movement</p>
            </div>
          </div>
        </div>


      </div>
    </div>


    <!-- Footer -->
    <div class="mt-8 text-center">
      <p class="text-gray-400 text-sm">
        🚗 Parking Robot System • Live Camera: {PI_IP}:8889 •
        <span class="text-cyan-400">{new Date().toLocaleString()}</span>
      </p>
    </div>
  </div>
</div>