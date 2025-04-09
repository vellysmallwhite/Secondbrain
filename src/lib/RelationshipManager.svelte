<script>
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";

  export let diaryId = "";
  export let diaries = [];

  let relationships = [];
  let isCreating = false;
  let selectedParentId = "";
  let selectedChildId = "";
  let relationshipType = "depends_on";
  let isLoading = false;
  let error = null;

  const dispatch = createEventDispatcher();

  async function loadRelationships() {
    try {
      isLoading = true;
      relationships = await invoke("get_relationships", { diary_id: diaryId });
    } catch (err) {
      console.error("Error loading relationships:", err);
      error = "Failed to load relationships";
    } finally {
      isLoading = false;
    }
  }

  async function createRelationship() {
    if (!selectedParentId || !selectedChildId || !relationshipType) {
      error = "Please select both parent and child diaries";
      return;
    }

    try {
      isLoading = true;
      await invoke("add_relationship", {
        parent_id: selectedParentId,
        child_id: selectedChildId,
        relationship_type: relationshipType
      });
      
      await loadRelationships();
      dispatch("relationshipCreated");
      isCreating = false;
      selectedParentId = "";
      selectedChildId = "";
      relationshipType = "depends_on";
    } catch (err) {
      console.error("Error creating relationship:", err);
      error = "Failed to create relationship";
    } finally {
      isLoading = false;
    }
  }

  async function deleteRelationship(id) {
    try {
      isLoading = true;
      await invoke("delete_relationship", { id });
      await loadRelationships();
      dispatch("relationshipDeleted");
    } catch (err) {
      console.error("Error deleting relationship:", err);
      error = "Failed to delete relationship";
    } finally {
      isLoading = false;
    }
  }

  $: if (diaryId) {
    loadRelationships();
  }
</script>

<div class="relationship-manager">
  <div class="header">
    <h3>Relationships</h3>
    <button 
      class="create-button" 
      on:click={() => isCreating = true}
      disabled={isLoading}
    >
      Create Relationship
    </button>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if isCreating}
    <div class="create-form">
      <div class="form-group">
        <label>Parent Diary</label>
        <select bind:value={selectedParentId}>
          <option value="">Select parent diary</option>
          {#each diaries as diary}
            {#if diary.id !== diaryId}
              <option value={diary.id}>{diary.title}</option>
            {/if}
          {/each}
        </select>
      </div>

      <div class="form-group">
        <label>Child Diary</label>
        <select bind:value={selectedChildId}>
          <option value="">Select child diary</option>
          {#each diaries as diary}
            {#if diary.id !== diaryId}
              <option value={diary.id}>{diary.title}</option>
            {/if}
          {/each}
        </select>
      </div>

      <div class="form-group">
        <label>Relationship Type</label>
        <select bind:value={relationshipType}>
          <option value="depends_on">Depends On</option>
          <option value="related_to">Related To</option>
          <option value="references">References</option>
        </select>
      </div>

      <div class="form-actions">
        <button 
          class="cancel-button" 
          on:click={() => isCreating = false}
          disabled={isLoading}
        >
          Cancel
        </button>
        <button 
          class="save-button" 
          on:click={createRelationship}
          disabled={isLoading}
        >
          {isLoading ? "Creating..." : "Create"}
        </button>
      </div>
    </div>
  {/if}

  <div class="relationships-list">
    {#if isLoading}
      <div class="loading">Loading relationships...</div>
    {:else if relationships.length === 0}
      <div class="empty-state">No relationships yet</div>
    {:else}
      {#each relationships as relationship}
        <div class="relationship-item">
          <div class="relationship-info">
            <span class="parent">
              {diaries.find(d => d.id === relationship.parent_id)?.title || "Unknown"}
            </span>
            <span class="relationship-type">{relationship.relationship_type}</span>
            <span class="child">
              {diaries.find(d => d.id === relationship.child_id)?.title || "Unknown"}
            </span>
          </div>
          <button 
            class="delete-button"
            on:click={() => deleteRelationship(relationship.id)}
            disabled={isLoading}
          >
            ×
          </button>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .relationship-manager {
    padding: 1rem;
    background-color: #f5f5f5;
    border-radius: 8px;
    margin-top: 1rem;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .header h3 {
    margin: 0;
    font-size: 1.2rem;
  }

  .create-button {
    background-color: #4CAF50;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    cursor: pointer;
  }

  .create-button:disabled {
    background-color: #cccccc;
    cursor: not-allowed;
  }

  .create-form {
    background-color: white;
    padding: 1rem;
    border-radius: 4px;
    margin-bottom: 1rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 500;
  }

  .form-group select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #ddd;
    border-radius: 4px;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .cancel-button {
    background-color: #f44336;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    cursor: pointer;
  }

  .save-button {
    background-color: #4CAF50;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    cursor: pointer;
  }

  .relationships-list {
    margin-top: 1rem;
  }

  .relationship-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    background-color: white;
    border-radius: 4px;
    margin-bottom: 0.5rem;
  }

  .relationship-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .relationship-type {
    color: #666;
    font-style: italic;
  }

  .delete-button {
    background: none;
    border: none;
    color: #f44336;
    font-size: 1.2rem;
    cursor: pointer;
    padding: 0.25rem;
  }

  .error {
    color: #f44336;
    margin-bottom: 1rem;
  }

  .loading, .empty-state {
    text-align: center;
    color: #666;
    padding: 1rem;
  }
</style> 