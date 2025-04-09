<script>
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";

  let container;
  let network;
  let graphData = { nodes: [], edges: [] };
  let loading = true;
  let error = null;
  let isCreatingRelationship = false;
  let selectedNodeId = null;
  let selectedEdge = null;
  let relationshipType = "depends_on";

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
        width: 2,
        shadow: true,
        smooth: {
          type: 'continuous'
        },
        font: {
          size: 12,
          align: 'middle'
        },
        arrows: {
          to: { enabled: true, scaleFactor: 0.5 }
        },
        color: {
          color: '#848484',
          highlight: '#1E88E5',
          hover: '#848484',
          inherit: false
        }
      },
      groups: {
        diary: {
          color: {
            background: '#4CAF50',
            border: '#2E7D32',
            highlight: { background: '#81C784', border: '#2E7D32' }
          },
          shape: 'dot'
        },
        tag: {
          color: {
            background: '#2196F3',
            border: '#1565C0',
            highlight: { background: '#64B5F6', border: '#1565C0' }
          },
          shape: 'diamond'
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

  function startCreatingRelationship() {
    isCreatingRelationship = true;
    selectedNodeId = null;
    if (network) {
      network.unselectAll();
    }
    console.log("Starting to create a relationship");
  }

  function cancelCreatingRelationship() {
    isCreatingRelationship = false;
    selectedNodeId = null;
    if (network) {
      network.unselectAll();
    }
    console.log("Cancelled relationship creation");
  }

  function deleteSelectedEdge() {
    if (selectedEdge) {
      const edge = graphData.edges.find(e => e.id === selectedEdge);
      if (edge) {
        console.log("Deleting relationship:", edge);
        invoke("delete_relationship", { id: edge.id })
          .then(() => {
            dispatch('relationshipDeleted');
            selectedEdge = null;
            refreshGraph();
          })
          .catch(err => {
            console.error("Error deleting relationship:", err);
            error = "Failed to delete relationship";
          });
      }
    }
  }

  // Update the network click handler
  function handleNetworkClick(params) {
    // Handle edge selection
    if (params.edges && params.edges.length > 0) {
      selectedEdge = params.edges[0];
      selectedNodeId = null;
      console.log("Selected edge:", selectedEdge);
      return;
    }
    
    // Handle node selection
    if (params.nodes && params.nodes.length > 0) {
      const nodeId = params.nodes[0];
      const node = graphData.nodes.find(n => n.id === nodeId);
      
      // Only allow diary nodes for relationships
      if (node && node.node_type === 'diary') {
        if (isCreatingRelationship) {
          if (selectedNodeId === null) {
            // First node selection (child)
            selectedNodeId = nodeId;
            console.log("Selected first node (child):", nodeId);
          } else if (selectedNodeId !== nodeId) {
            // Second node selection (parent)
            console.log("Creating relationship:", {
              childId: selectedNodeId,
              parentId: nodeId,
              relationshipType
            });
            
            // Create the relationship
            invoke("add_relationship", {
              parent_id: nodeId,
              child_id: selectedNodeId,
              relationship_type: relationshipType
            })
            .then(() => {
              console.log("Relationship created successfully");
              // Keep relationship mode active but reset selection
              selectedNodeId = null;
              if (network) network.unselectAll();
              refreshGraph();
              dispatch('relationshipCreated');
            })
            .catch(err => {
              console.error("Error creating relationship:", err);
              error = "Failed to create relationship: " + err;
            });
          }
        } else {
          // Normal node selection
          dispatch('selectDiary', { id: nodeId });
        }
      } else if (node && node.node_type === 'tag') {
        dispatch('selectTag', { name: node.label });
      }
    } else {
      // Clicked on empty space
      selectedEdge = null;
      if (!isCreatingRelationship) {
        selectedNodeId = null;
      }
    }
  }

  async function handleCreateRelationship(event) {
    const { parentId, childId, relationshipType } = event.detail;
    
    try {
      await invoke("add_relationship", {
        parent_id: parentId,
        child_id: childId,
        relationship_type: relationshipType
      });
      
      // Refresh the graph
      await loadGraphData();
    } catch (err) {
      console.error("Error creating relationship:", err);
      error = "Failed to create relationship";
    }
  }

  async function handleRelationshipDeleted() {
    // Refresh the graph
    await refreshGraph();
  }
</script>

<div class="graph-container">
  {#if loading}
    <div class="loading">Loading graph visualization...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else}
    <div class="graph-controls">
      <button on:click={startCreatingRelationship} disabled={isCreatingRelationship}>
        Create Relationship
      </button>
      
      {#if isCreatingRelationship}
        <select bind:value={relationshipType}>
          <option value="depends_on">Depends On</option>
          <option value="related_to">Related To</option>
          <option value="references">References</option>
        </select>
        
        <button on:click={cancelCreatingRelationship}>
          Cancel
        </button>
        
        <div class="relationship-helper">
          {#if selectedNodeId === null}
            Select first node (child/son/daughter)
          {:else}
            Now select second node (parent/father/mother)
          {/if}
        </div>
      {/if}
      
      {#if selectedEdge}
        <button on:click={deleteSelectedEdge} class="delete-button">
          Delete Relationship
        </button>
      {/if}
    </div>
    
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
  
  .graph-controls {
    display: flex;
    gap: 10px;
    margin-bottom: 10px;
    align-items: center;
  }
  
  .delete-button {
    background-color: #f44336;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    cursor: pointer;
  }
  
  .relationship-helper {
    background-color: #e3f2fd;
    padding: 8px;
    border-radius: 4px;
    border: 1px solid #bbdefb;
  }
</style> 