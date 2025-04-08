<script>
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";

  let container;
  let network;
  let graphData = { nodes: [], edges: [] };
  let loading = true;
  let error = null;

  const dispatch = createEventDispatcher();

  onMount(async () => {
    try {
      // Load vis-network script dynamically
      const script = document.createElement('script');
      script.src = 'https://unpkg.com/vis-network/standalone/umd/vis-network.min.js';
      script.async = true;
      
      script.onload = async () => {
        await loadGraphData();
        initializeNetwork();
      };
      
      document.head.appendChild(script);
      
      // Load vis-network CSS
      const link = document.createElement('link');
      link.rel = 'stylesheet';
      link.href = 'https://unpkg.com/vis-network/styles/vis-network.min.css';
      document.head.appendChild(link);
      
      return () => {
        document.head.removeChild(script);
        document.head.removeChild(link);
        if (network) {
          network.destroy();
        }
      };
    } catch (err) {
      console.error("Error loading vis-network:", err);
      error = "Failed to load visualization library";
      loading = false;
    }
  });

  async function loadGraphData() {
    try {
      loading = true;
      const data = await invoke("get_graph_data");
      
      // Transform data for vis.js
      const nodes = data.nodes.map(node => ({
        id: node.id,
        label: node.label,
        group: node.node_type,
        title: getNodeTooltip(node),
      }));
      
      const edges = data.edges.map(edge => ({
        id: edge.id,
        from: edge.source,
        to: edge.target,
        label: edge.label.replace('tagged_as_', ''),
        arrows: 'to',
      }));
      
      graphData = { nodes, edges };
      loading = false;
    } catch (err) {
      console.error("Error loading graph data:", err);
      error = "Failed to load graph data";
      loading = false;
    }
  }

  function getNodeTooltip(node) {
    if (node.node_type === 'diary') {
      return `Title: ${node.label}<br>Created: ${new Date(node.properties.created_at).toLocaleString()}`;
    } else {
      return `Tag: ${node.label}`;
    }
  }

  function initializeNetwork() {
    if (!container || !window.vis) return;
    
    const nodes = new window.vis.DataSet(graphData.nodes);
    const edges = new window.vis.DataSet(graphData.edges);
    
    const data = { nodes, edges };
    
    const options = {
      nodes: {
        shape: 'dot',
        size: 16,
        font: {
          size: 12,
          face: 'Tahoma'
        },
        borderWidth: 2,
        shadow: true
      },
      edges: {
        width: 1,
        shadow: true,
        smooth: {
          type: 'continuous'
        },
        font: {
          size: 10,
          align: 'middle'
        },
        color: {
          color: '#848484',
          highlight: '#1E88E5',
          hover: '#848484'
        }
      },
      groups: {
        diary: {
          color: {
            background: '#4CAF50',
            border: '#2E7D32',
            highlight: { background: '#81C784', border: '#2E7D32' }
          }
        },
        tag: {
          color: {
            background: '#2196F3',
            border: '#1565C0',
            highlight: { background: '#64B5F6', border: '#1565C0' }
          }
        }
      },
      physics: {
        stabilization: {
          iterations: 100
        },
        barnesHut: {
          gravitationalConstant: -2000,
          centralGravity: 0.3,
          springLength: 95,
          springConstant: 0.04,
          damping: 0.09
        }
      },
      interaction: {
        hover: true,
        tooltipDelay: 200,
        hideEdgesOnDrag: true,
        navigationButtons: true
      }
    };
    
    network = new window.vis.Network(container, data, options);
    
    network.on("click", function(params) {
      if (params.nodes.length > 0) {
        const nodeId = params.nodes[0];
        const node = graphData.nodes.find(n => n.id === nodeId);
        
        if (node && node.group === 'diary') {
          dispatch('selectDiary', { id: nodeId });
        } else if (node && node.group === 'tag') {
          dispatch('selectTag', { name: node.label });
        }
      }
    });
  }

  export async function refreshGraph() {
    await loadGraphData();
    if (network) {
      network.setData({
        nodes: new window.vis.DataSet(graphData.nodes),
        edges: new window.vis.DataSet(graphData.edges)
      });
    }
  }
</script>

<div class="graph-container">
  {#if loading}
    <div class="loading">Loading graph visualization...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else}
    <div class="graph-canvas" bind:this={container}></div>
  {/if}
</div>

<style>
  .graph-container {
    width: 100%;
    height: 100%;
    min-height: 500px;
    background-color: #f9f9f9;
    border-radius: 8px;
    overflow: hidden;
    position: relative;
  }
  
  .graph-canvas {
    width: 100%;
    height: 100%;
  }
  
  .loading, .error {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    text-align: center;
  }
  
  .error {
    color: #d32f2f;
  }
</style> 