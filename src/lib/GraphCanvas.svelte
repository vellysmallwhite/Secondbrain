<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";
  import { v4 as uuidv4 } from 'uuid';

  // @ts-ignore
  let container;
  // @ts-ignore
  let network;
  let graphData = { nodes: [], edges: [] };
  let loading = true;
  let error = null;
  let isCreatingRelationship = false;
  let hasSelectedFirstNode = false;
  let selectedNodeId = null;
  let selectedEdge = null;
  let relationshipType = "depends_on";
  let statusMessage = null;
  let isInitialized = false;
  
  // Debug state
  let showDebug = false;
  let debugMessages = [];
  
  // Add these global variables
  let nodesDataset;
  let edgesDataset;
  
  function debug(message, data = null) {
    const timestamp = new Date().toLocaleTimeString();
    const logMessage = data 
      ? `${timestamp}: ${message} - ${JSON.stringify(data)}`
      : `${timestamp}: ${message}`;
      
    debugMessages = [...debugMessages.slice(-19), logMessage];
    console.log(`[DEBUG] ${message}`, data || '');
  }

  const dispatch = createEventDispatcher();

  onMount(() => {
    debug("GraphCanvas mounted");
    console.log("GraphCanvas component mounted");
    
    // Add keyboard shortcut for developer tools and debug overlay
    window.addEventListener('keydown', (e) => {
      // Ctrl+Shift+I for dev tools
      if (e.ctrlKey && e.shiftKey && e.key === 'I') {
        if (window.__TAURI_INVOKE__) {
          window.__TAURI_INVOKE__('open_devtools');
        }
      }
      
      // Ctrl+Shift+D to toggle debug overlay
      if (e.ctrlKey && e.shiftKey && e.key === 'D') {
        showDebug = !showDebug;
        debug(`Debug overlay ${showDebug ? 'enabled' : 'disabled'}`);
      }
    });
    
    try {
      // Load vis-network script dynamically
      const script = document.createElement('script');
      script.src = 'https://unpkg.com/vis-network/standalone/umd/vis-network.min.js';
      script.async = true;
      
      script.onload = async () => {
        await loadGraphData();
        initializeNetwork();
        isInitialized = true;
        debug("Graph component fully initialized");
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

  // Safe invoke function to prevent calls with empty parameters
  async function safeInvoke(command, params) {
    
      // For relationship creation, verify we have the required parameters
      // if (!params || !params.parent_id || !params.child_id) {
      //   const errorMessage = "Cannot create relationship: Missing required parameters";
      //   debug("BLOCKED API CALL", { command, params, reason: errorMessage });
      //   throw new Error(errorMessage);
      // }
    
    
    debug(`Invoking from safe invoke ${command}`, params || {});

    if (command === 'add_relationship') {await new Promise(resolve => setTimeout(resolve, 10000)); }
    return invoke(command, params);
  }

  async function loadGraphData() {
    try {
      loading = true;
      const data = await safeInvoke("get_graph_data");
      
      debug("Received graph data:", {
        nodes: data.nodes.length,
        edges: data.edges.length
      });
      
      // Transform data for vis.js
      // @ts-ignore
      const processedNodes = data.nodes.map(node => {
        // Ensure node_type is set
        if (!node.node_type) {
          debug("WARNING: Node missing node_type", node);
        }
        
        return {
        id: node.id,
        label: node.label,
          group: node.node_type, // This becomes the group property used for styling
          node_type: node.node_type, // Keep original for reference
        title: getNodeTooltip(node),
        };
      });
      
      // Check for diary nodes
      const diaryNodeCount = processedNodes.filter(n => n.group === 'diary').length;
      debug("Diary nodes count:", diaryNodeCount);
      
      // @ts-ignore
      const processedEdges = data.edges.map(edge => ({
        id: edge.id,
        from: edge.source,
        to: edge.target,
        label: edge.label.replace('tagged_as_', ''),
        arrows: 'to',
      }));
      
      // Store in graphData for initial reference and for network initialization
      graphData = { nodes: processedNodes, edges: processedEdges };
      
      // If datasets are already initialized, update them too
      if (nodesDataset && edgesDataset) {
        nodesDataset.clear();
        edgesDataset.clear();
        nodesDataset.add(processedNodes);
        edgesDataset.add(processedEdges);
      }
      
      debug("Graph data processed", { 
        nodeCount: processedNodes.length, 
        edgeCount: processedEdges.length,
        groups: [...new Set(processedNodes.map(n => n.group))]
      });
      
      loading = false;
    } catch (err) {
      console.error("Error loading graph data:", err);
      error = "Failed to load graph data";
      loading = false;
    }
  }

  // @ts-ignore
  function getNodeTooltip(node) {
    if (node.node_type === 'diary') {
      return `Title: ${node.label}<br>Created: ${new Date(node.properties.created_at).toLocaleString()}`;
    } else {
      return `Tag: ${node.label}`;
    }
  }

  function initializeNetwork() {
    // @ts-ignore
    if (!container || !window.vis) {
      debug("Cannot initialize network - container or vis.js not available");
      return;
    }
    
    debug("Initializing network");
    
    try {
      // @ts-ignore
      nodesDataset = new window.vis.DataSet(graphData.nodes);
      // @ts-ignore
      edgesDataset = new window.vis.DataSet(graphData.edges);
      
      debug("Created data sets", { 
        nodesCount: nodesDataset.length, 
        edgesCount: edgesDataset.length 
      });
      
      const data = { nodes: nodesDataset, edges: edgesDataset };
      
      // Enhanced options to make nodes more selectable
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
          navigationButtons: true,
          selectable: true,  // Ensure nodes are selectable
          selectConnectedEdges: false,  // Don't auto-select edges
          hoverConnectedEdges: true,
          multiselect: false  // Only allow selecting one node at a time
        }
      };
      
      // @ts-ignore
    network = new window.vis.Network(container, data, options);
    
      // Connect all event handlers
      // Click event
    network.on("click", function(params) {
        debug("Network click detected", params);
        handleNetworkClick(params);
      });
      
      // Add more event listeners for debugging
      network.on("selectNode", function(params) {
        debug("Node selected event", params);
      });
      
      network.on("selectEdge", function(params) {
        debug("Edge selected event", params);
      });
      
      network.on("hoverNode", function(params) {
        debug("Node hover event", { nodeId: params.node });
      });
      
      debug("Network initialized with " + graphData.nodes.length + " nodes and " + graphData.edges.length + " edges");
    } catch (err) {
      debug("Error initializing network", err);
      error = "Failed to initialize network: " + err;
    }
  }

  export async function updateGraph(showLoading = false) {
    try {
      // Only show loading if explicitly requested
      if (showLoading) loading = true;
      
      // Get updated data
      const data = await safeInvoke("get_graph_data");
      
      debug("Received updated graph data:", {
        nodes: data.nodes.length,
        edges: data.edges.length
      });
      
      // Process nodes and edges (same logic as loadGraphData)
      // @ts-ignore
      const processedNodes = data.nodes.map(node => ({
        id: node.id,
        label: node.label,
        group: node.node_type,
        node_type: node.node_type,
        title: getNodeTooltip(node),
      }));
      
      // @ts-ignore
      const processedEdges = data.edges.map(edge => ({
        id: edge.id,
        from: edge.source,
        to: edge.target,
        label: edge.label.replace('tagged_as_', ''),
        arrows: 'to',
      }));
      
      // Update datasets if network is initialized
      if (nodesDataset && edgesDataset && network) {
        // Find nodes/edges to remove
        const currentNodeIds = nodesDataset.getIds();
        const newNodeIds = processedNodes.map(n => n.id);
        const nodesToRemove = currentNodeIds.filter(id => !newNodeIds.includes(id));
        
        const currentEdgeIds = edgesDataset.getIds();
        const newEdgeIds = processedEdges.map(e => e.id);
        const edgesToRemove = currentEdgeIds.filter(id => !newEdgeIds.includes(id));
        
        // Remove deleted items first
        if (nodesToRemove.length > 0) nodesDataset.remove(nodesToRemove);
        if (edgesToRemove.length > 0) edgesDataset.remove(edgesToRemove);
        
        // Update/add nodes and edges
        nodesDataset.update(processedNodes);
        edgesDataset.update(processedEdges);
        
        debug("Graph updated smoothly", { 
          nodesUpdated: processedNodes.length,
          edgesUpdated: processedEdges.length,
          nodesRemoved: nodesToRemove.length,
          edgesRemoved: edgesToRemove.length
        });
      } else {
        // If datasets aren't initialized yet, just update graphData
        graphData = { nodes: processedNodes, edges: processedEdges };
        debug("Updated graphData only, network not initialized");
      }
      
      // Always update the internal graphData for consistency
      graphData = { nodes: processedNodes, edges: processedEdges };
    } catch (err) {
      console.error("Error updating graph:", err);
      error = "Failed to update graph";
    } finally {
      if (showLoading) loading = false;
    }
  }

  function startCreatingRelationship() {
    debug("Start creating relationship called");
    console.log("Starting relationship creation process");
    
    // Reset state first
    selectedNodeId = null;
    hasSelectedFirstNode = false;
    statusMessage = null;
    selectedEdge = null;  // Also clear any selected edge
    
    // Set creation mode
    isCreatingRelationship = true;
    
    // Clear network selection
    if (network) {
      debug("Clearing network selection");
      try {
        network.unselectAll();
        
        // Enable node selection mode
        network.setOptions({
          interaction: {
            selectable: true,
            selectConnectedEdges: false,
            multiselect: false
          }
        });
      } catch (err) {
        debug("Error in network configuration", err);
      }
    }
    
    debug("Relationship creation mode activated", { 
      isCreatingRelationship: true, 
      hasSelectedFirstNode: false, 
      selectedNodeId: null, 
      relationshipType 
    });
    
    // Display instructions
    statusMessage = "Click on the first node (child/son/daughter)";
  }

  function cancelCreatingRelationship() {
    isCreatingRelationship = false;
    hasSelectedFirstNode = false;
    selectedNodeId = null;
    statusMessage = null;
    if (network) {
      network.unselectAll();
    }
    debug("Cancelled relationship creation mode", { 
      isCreatingRelationship, 
      hasSelectedFirstNode, 
      selectedNodeId
    });
  }

  /**
   * Reset relationship creation state
   */
  function resetRelationshipState() {
    hasSelectedFirstNode = false;
    selectedNodeId = null;
    if (network) network.unselectAll();
    debug("Reset relationship state", { 
      hasSelectedFirstNode, 
      selectedNodeId 
    });
  }

  function deleteSelectedEdge() {
    if (selectedEdge) {
      // @ts-ignore
      const edge = graphData.edges.find(e => e.id === selectedEdge);
      if (edge) {
        debug("Deleting relationship", edge);
        safeInvoke("delete_relationship", { id: edge.id })
          .then(() => {
            dispatch('relationshipDeleted');
            selectedEdge = null;
            updateGraph();
          })
          .catch(err => {
            debug("Error deleting relationship", err);
            error = "Failed to delete relationship";
          });
      }
    }
  }

  /**
   * Check if a relationship already exists between two nodes
   */
  function relationshipExists(node1Id, node2Id) {
    // @ts-ignore
    const exists = graphData.edges.some(edge => 
      (edge.from === node1Id && edge.to === node2Id) || 
      (edge.from === node2Id && edge.to === node1Id)
    );
    debug("Checking if relationship exists", { node1Id, node2Id, exists });
    return exists;
  }

  /**
   * Handle network click events
   */
  // @ts-ignore
  async function handleNetworkClick(params) {
    debug("Network click event raw params", params);
    
    // Enhanced logging to see exactly what's in the params
    debug("Params structure", {
      hasNodes: !!params.nodes, 
      nodesLength: params.nodes ? params.nodes.length : 'N/A',
      hasEdges: !!params.edges,
      edgesLength: params.edges ? params.edges.length : 'N/A',
      pointer: params.pointer,
      event: params.event ? params.event.type : 'N/A'
    });
    
    debug("Current state", { 
      isCreatingRelationship, 
      hasSelectedFirstNode, 
      selectedNodeId,
      selectedEdge
    });
    
    // First handle edge selection (no change here)
    if (params.edges && params.edges.length > 0) {
      selectedEdge = params.edges[0];
      selectedNodeId = null;
      hasSelectedFirstNode = false;
      debug("Selected edge", selectedEdge);
      return;
    }
    
    // Improved node selection check
    // Check if we have a node ID in the params
    const clickedNodeId = params.nodes && params.nodes.length > 0 ? params.nodes[0] : null;
    
    if (!clickedNodeId) {
      debug("No node detected in click event");
      // Clicked on empty space
      selectedEdge = null;
      if (!isCreatingRelationship) {
        selectedNodeId = null;
        hasSelectedFirstNode = false;
      }
      return;
    }
    
    debug("Clicked on node ID:", clickedNodeId);
    
    // Find the node in our data
    // @ts-ignore
    const node = graphData.nodes.find(n => n.id === clickedNodeId);
    
    if (!node) {
      debug("WARNING: Node not found in graphData!");
      return;
    }
    
    // Log detailed node information for debugging
    debug("Node details", { 
      id: node.id,
      label: node.label,
      group: node.group, 
      nodeType: node.node_type || node.group, // Fallback to group if node_type is missing
      isCreatingRelationship,
      hasSelectedFirstNode
    });
    
    // Try multiple ways to determine if this is a diary node
    const isDiaryNode = 
      (node.group === 'diary') || 
      (node.node_type === 'diary') || 
      (typeof node.properties === 'object' && node.properties && 'created_at' in node.properties);
    
    debug("Node type check", { 
      isDiaryNode, 
      group: node.group, 
      node_type: node.node_type
    });
    
    // Only allow diary nodes for relationships
    if (isDiaryNode) {
      if (isCreatingRelationship) {
        debug("In relationship creation mode with diary node");
        
        // First selection
        if (!hasSelectedFirstNode) {
          debug("This is the first node selection");
          selectedNodeId = clickedNodeId;
          hasSelectedFirstNode = true;
          statusMessage = null;
          
          debug("Updated selection state", { 
            nodeId: clickedNodeId, 
            hasSelectedFirstNode: true, 
            selectedNodeId: clickedNodeId
          });
        } 
        // Second selection
        else if (selectedNodeId !== clickedNodeId) {

          const firstNodeId = selectedNodeId;
          debug("This is the second node selection", { 
            firstNodeId, 
            secondNodeId: clickedNodeId 
          });

          
          //await new Promise(resolve => setTimeout(resolve, 100000));

          
          // Add detailed info about the node
          
          
          // Create the relationship
          try {
            // Triple check parameters before invoking
            if (!clickedNodeId || typeof clickedNodeId !== 'string' || clickedNodeId.trim() === '') {
              throw new Error("Invalid parent ID");
            }
            
            if (!firstNodeId || typeof firstNodeId !== 'string' || firstNodeId.trim() === '') {
              throw new Error("Invalid child ID");
            }
            
            // Generate a UUID for the relationship
            const relationshipId = uuidv4();
            
            const params = {
              id: relationshipId,
              parentId: clickedNodeId,
              childId: firstNodeId,
              relationshipType: relationshipType
            };
            
            debug("Invoking add_relationship with:", params);
            console.log("Creating relationship with params:", params);
            
            // Immediately update the graph with the new relationship
            if (nodesDataset && edgesDataset && network) {
              // Create a new edge with the relationship data
              const newEdge = {
                id: relationshipId,
                from: params.parentId,
                to: params.childId,
                label: params.relationshipType,
                arrows: 'to'
              };
              
              // Add the new edge to the edges dataset
              edgesDataset.add(newEdge);
              
              // Also update our internal graphData to keep it consistent
              graphData.edges.push({
                id: relationshipId,
                source: params.parentId,
                target: params.childId,
                label: params.relationshipType
              });
              
              debug("Locally added new relationship to graph", newEdge);
            }
            
            // Send the request to the backend (we don't need to wait for the result)
            safeInvoke("add_relationship", params)
              .then(result => {
                console.log('Relationship created successfully:', result);
                debug("Relationship created successfully", result);
                statusMessage = "Relationship created successfully";
              })
              .catch(err => {
                console.error("Failed to create relationship:", err);
                debug("Error creating relationship", err);
                
                // If there was an error, remove the edge we added
                if (nodesDataset && edgesDataset) {
                  try {
                    edgesDataset.remove(relationshipId);
                    graphData.edges = graphData.edges.filter(edge => edge.id !== relationshipId);
                    debug("Removed edge after failed relationship creation");
                  } catch (e) {
                    debug("Error removing edge after failed creation:", e);
                  }
                }
                
                // More detailed error message
                const errorMsg = typeof err === 'object' ? JSON.stringify(err) : String(err);
                statusMessage = "Failed to create relationship: " + errorMsg;
                error = statusMessage;
              });
            
            // Keep relationship mode active but reset selection
            resetRelationshipState();
            
            // Dispatch event after graph update completes
            dispatch('relationshipCreated');
            
          } catch (err) {
            console.error("Failed to create relationship:", err);
            debug("Error creating relationship", err);
            
            // More detailed error message
            const errorMsg = typeof err === 'object' ? JSON.stringify(err) : String(err);
            statusMessage = "Failed to create relationship: " + errorMsg;
            error = statusMessage;
          }
        } else {
          // Selected same node twice
          statusMessage = "Please select a different node for the second selection";
          debug(statusMessage);
        }
      } else {
        // Normal node selection
        debug("Normal diary node selection", { nodeId: clickedNodeId });
        dispatch('selectDiary', { id: clickedNodeId });
      }
    } else {
      // Check if this is a tag node
      const isTagNode = 
        (node.group === 'tag') || 
        (node.node_type === 'tag');
      
      if (isTagNode) {
        if (isCreatingRelationship) {
          statusMessage = "Cannot create relationships with tag nodes";
          debug(statusMessage);
        } else {
          debug("Normal tag node selection", { name: node.label });
          dispatch('selectTag', { name: node.label });
        }
      } else {
        debug("WARNING: Unknown node type", { 
          group: node.group,
          node_type: node.node_type
        });
      }
    }
  }

  async function handleRelationshipDeleted() {
    debug("Relationship deleted");
    // Refresh the graph
    await updateGraph();
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
          {#if !hasSelectedFirstNode}
            Select first node (child/son/daughter)
          {:else}
            Now select second node (parent/father/mother)
          {/if}
          
          {#if statusMessage}
            <div class="status-message">{statusMessage}</div>
          {/if}
        </div>
      {/if}
      
      {#if selectedEdge}
        <button on:click={deleteSelectedEdge} class="delete-button">
          Delete Relationship
        </button>
      {/if}
      
      <button class="debug-toggle" on:click={() => showDebug = !showDebug}>
        {showDebug ? 'Hide Debug' : 'Show Debug'}
      </button>
    </div>
    
    <div class="graph-canvas" bind:this={container}></div>
    
    {#if showDebug}
      <div class="debug-overlay">
        <h3>Debug Log (Press Ctrl+Shift+D to toggle)</h3>
        <ul>
          {#each debugMessages as message}
            <li>{message}</li>
          {/each}
        </ul>
      </div>
    {/if}
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
  
  .status-message {
    margin-top: 8px;
    font-style: italic;
    color: #d32f2f;
    font-size: 0.9rem;
  }
  
  .debug-toggle {
    margin-left: auto;
    background-color: #212121;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    cursor: pointer;
  }
  
  .debug-overlay {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background-color: rgba(0, 0, 0, 0.8);
    color: #00ff00;
    font-family: monospace;
    padding: 1rem;
    max-height: 300px;
    overflow-y: auto;
    z-index: 1000;
  }
  
  .debug-overlay h3 {
    margin-top: 0;
    color: white;
  }
  
  .debug-overlay ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  
  .debug-overlay li {
    margin-bottom: 0.25rem;
    border-bottom: 1px solid #333;
    padding-bottom: 0.25rem;
  }
</style> 