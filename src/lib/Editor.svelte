<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { createEventDispatcher } from "svelte";

  export let entry = { id: "", title: "", content: "", tags: [] };
  export let isNew = true;

  let title = entry.title || "";
  let content = entry.content || "";
  let tags = entry.tags || [];
  let tagInput = "";
  let isSaving = false;
  let saveStatus = "";
  let hasUnsavedChanges = false;

  const dispatch = createEventDispatcher();

  // 添加响应式声明，监听 entry 变化
  $: if (entry) {
    // Always update local state when entry changes
    title = entry.title || "";
    content = entry.content || "";
    tags = [...(entry.tags || [])];
    isNew = !entry.id;
    hasUnsavedChanges = false;
  }

  // 监听内容变化
  $: {
    if (entry && entry.id) {
      hasUnsavedChanges = 
        title !== entry.title || 
        content !== entry.content || 
        JSON.stringify(tags) !== JSON.stringify(entry.tags);
    } else {
      hasUnsavedChanges = title.trim() !== "" || content.trim() !== "" || tags.length > 0;
    }
  }

  function addTag() {
    const trimmedTag = tagInput.trim();
    if (trimmedTag && !tags.includes(trimmedTag)) {
      tags = [...tags, trimmedTag];
      tagInput = "";
    }
  }

  function removeTag(tag) {
    tags = tags.filter(t => t !== tag);
  }

  function handleKeydown(event) {
    if (event.key === "Enter") {
      event.preventDefault();
      addTag();
    }
  }

  async function saveEntry() {
    if (!title.trim()) {
      saveStatus = "Please add a title";
      return;
    }
    
    if (!hasUnsavedChanges) {
      saveStatus = "No changes to save";
      return;
    }
    
    isSaving = true;
    
    try {
      if (isNew) {
        // 检查标题是否已存在
        const existingDiaries = await invoke("list_diaries");
        const titleExists = existingDiaries.some(diary => 
          diary.title.toLowerCase() === title.toLowerCase() && diary.id !== entry.id
        );
        
        if (titleExists) {
          saveStatus = "Title already exists";
          return;
        }
      }
      
      const id = await invoke("save_diary", {
        id: entry.id || null,
        title,
        content,
        tags
      });
      
      saveStatus = "Saved";
      hasUnsavedChanges = false;
      
      if (isNew) {
        isNew = false;
        entry.id = id;
        dispatch("saved", { id });
      }
    } catch (error) {
      console.error("Error saving diary:", error);
      saveStatus = "Error saving";
    } finally {
      isSaving = false;
    }
  }
</script>

<div class="editor">
  <div class="editor-header">
    <input 
      type="text" 
      placeholder="Title" 
      bind:value={title} 
      class="title-input"
    />
    <div class="save-status">{saveStatus}</div>
  </div>
  
  <div class="tags-container">
    <div class="tags-list">
      {#each tags as tag}
        <span class="tag">
          {tag}
          <button class="tag-remove" on:click={() => removeTag(tag)}>×</button>
        </span>
      {/each}
    </div>
    
    <div class="tag-input-container">
      <input 
        type="text" 
        placeholder="Add tag..." 
        bind:value={tagInput} 
        on:keydown={handleKeydown}
        class="tag-input"
      />
      <button class="tag-add" on:click={addTag}>+</button>
    </div>
  </div>
  
  <textarea 
    placeholder="Write your thoughts..." 
    bind:value={content} 
    class="content-textarea"
  ></textarea>

  <div class="save-button-container">
    <button 
      class="save-button" 
      on:click={saveEntry}
      disabled={isSaving}
    >
      {isSaving ? "Saving..." : "Save"}
    </button>
  </div>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 1rem;
    background-color: #fff;
    border-radius: 8px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }
  
  .editor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }
  
  .title-input {
    font-size: 1.5rem;
    font-weight: bold;
    border: none;
    outline: none;
    width: 100%;
    padding: 0.5rem 0;
    border-bottom: 1px solid #eee;
  }
  
  .save-status {
    font-size: 0.8rem;
    color: #888;
    min-width: 80px;
    text-align: right;
  }
  
  .tags-container {
    margin-bottom: 1rem;
  }
  
  .tags-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  
  .tag {
    display: inline-flex;
    align-items: center;
    background-color: #f0f0f0;
    border-radius: 16px;
    padding: 0.25rem 0.75rem;
    font-size: 0.8rem;
  }
  
  .tag-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1rem;
    margin-left: 0.25rem;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #888;
  }
  
  .tag-input-container {
    display: flex;
    align-items: center;
  }
  
  .tag-input {
    flex: 1;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 0.5rem;
    font-size: 0.9rem;
  }
  
  .tag-add {
    background-color: #4a86e8;
    color: white;
    border: none;
    border-radius: 4px;
    width: 2rem;
    height: 2rem;
    margin-left: 0.5rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
  }
  
  .content-textarea {
    flex: 1;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 1rem;
    font-size: 1rem;
    line-height: 1.5;
    resize: none;
    min-height: 200px;
    margin-bottom: 1rem;
  }

  .save-button-container {
    display: flex;
    justify-content: flex-end;
  }

  .save-button {
    background-color: #4CAF50;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1.5rem;
    font-size: 1rem;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .save-button:hover {
    background-color: #45a049;
  }

  .save-button:disabled {
    background-color: #cccccc;
    cursor: not-allowed;
  }
</style> 