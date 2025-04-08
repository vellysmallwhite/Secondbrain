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
  let saveTimeout;
  let isSaving = false;
  let saveStatus = "";

  const dispatch = createEventDispatcher();

  // 添加响应式声明，监听 entry 变化
  $: if (entry && entry.id) {
    // 保存当前编辑内容（如果有未保存的修改）
    if (!isNew && title && content && title !== entry.title || content !== entry.content || JSON.stringify(tags) !== JSON.stringify(entry.tags)) {
      saveCurrentEntry();
    }
    
    // 更新编辑器内容为新选择的日记
    title = entry.title || "";
    content = entry.content || "";
    tags = [...(entry.tags || [])];
    isNew = false;
  }

  onMount(() => {
    if (!isNew) {
      title = entry.title;
      content = entry.content;
      tags = [...entry.tags];
    }
  });

  onDestroy(() => {
    if (saveTimeout) {
      clearTimeout(saveTimeout);
    }
  });

  function addTag() {
    if (tagInput.trim() && !tags.includes(tagInput.trim())) {
      tags = [...tags, tagInput.trim()];
      tagInput = "";
      debounceSave();
    }
  }

  function removeTag(tag) {
    tags = tags.filter(t => t !== tag);
    debounceSave();
  }

  function handleKeydown(event) {
    if (event.key === "Enter") {
      event.preventDefault();
      addTag();
    }
  }

  function handleInput() {
    debounceSave();
  }

  function debounceSave() {
    if (saveTimeout) {
      clearTimeout(saveTimeout);
    }
    
    saveStatus = "Saving...";
    
    saveTimeout = setTimeout(() => {
      saveEntry();
    }, 1000);
  }

  async function saveEntry() {
    if (!title.trim()) {
      saveStatus = "Please add a title";
      return;
    }
    
    isSaving = true;
    
    try {
      const id = await invoke("save_diary", {
        title,
        content,
        tags
      });
      
      saveStatus = "Saved";
      
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

  // 保存当前编辑的内容
  async function saveCurrentEntry() {
    if (saveTimeout) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }
    
    if (!title.trim()) return;
    
    try {
      await saveEntry();
    } catch (error) {
      console.error("Failed to save entry before switching:", error);
    }
  }
</script>

<div class="editor">
  <div class="editor-header">
    <input 
      type="text" 
      placeholder="Title" 
      bind:value={title} 
      on:input={handleInput}
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
    on:input={handleInput}
    class="content-textarea"
  ></textarea>
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
  }
</style> 