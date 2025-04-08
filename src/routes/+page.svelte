<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Editor from "../lib/Editor.svelte";
  import GraphCanvas from "../lib/GraphCanvas.svelte";

  /**
   * @typedef {Object} DiaryEntry
   * @property {string} id - 日记ID
   * @property {string} title - 标题
   * @property {string} content - 内容
   * @property {string[]} tags - 标签
   * @property {string} created_at - 创建时间
   * @property {string} updated_at - 更新时间
   */

  /** @type {DiaryEntry[]} */
  let diaries = [];
  /** @type {DiaryEntry|null} */
  let selectedDiary = null;
  let isLoading = true;
  /** @type {string|null} */
  let error = null;
  let activeTab = "editor";
  /** @type {any} */
  let graphCanvas;
  /** @type {string|null} */
  let currentTag = null;

  /** @type {Map<string, DiaryEntry>} */
  const diaryCache = new Map();

  onMount(async () => {
    await loadDiaries();
  });

  async function loadDiaries() {
    try {
      isLoading = true;
      /** @type {DiaryEntry[]} */
      const result = await invoke("list_diaries");
      // 确保按时间倒序排列（最新的在最前面）
      diaries = result.sort((a, b) => 
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      );
      
      // 更新缓存
      diaries.forEach(diary => {
        diaryCache.set(diary.id, diary);
      });
      
      isLoading = false;
    } catch (err) {
      console.error("Error loading diaries:", err);
      error = "Failed to load diaries";
      isLoading = false;
    }
  }

  /**
   * 重置过滤器，显示所有日记
   */
  async function resetFilter() {
    currentTag = null;
    await loadDiaries();
  }

  async function handleDiarySaved(event) {
    await loadDiaries();
    if (graphCanvas) {
      await graphCanvas.refreshGraph();
    }
  }

  function createNewDiary() {
    selectedDiary = null;
    activeTab = "editor";
  }

  /**
   * 处理日记选择
   * @param {{ detail: { id: string } }} event
   */
  async function handleSelectDiary(event) {
    try {
      const id = event.detail.id;
      
      // 先检查缓存中是否有完整的日记内容
      const cachedDiary = diaryCache.get(id);
      if (cachedDiary && cachedDiary.content) {
        selectedDiary = cachedDiary;
      } else {
        // 如果缓存中没有或内容不完整，则从后端获取
        const diary = await invoke("get_diary", { id });
        
        // 更新缓存
        diaryCache.set(id, diary);
        selectedDiary = diary;
      }
      
      activeTab = "editor";
    } catch (err) {
      console.error("Error loading diary:", err);
      error = "Failed to load diary";
    }
  }

  /**
   * 处理标签选择
   * @param {{ detail: { name: string } }} event
   */
  async function handleSelectTag(event) {
    try {
      const tag = event.detail.name;
      currentTag = tag;  // 更新当前选中的标签
      const result = await invoke("search_diaries_by_tag", { tag });
      diaries = result.sort((a, b) => 
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      );
    } catch (err) {
      console.error("Error searching diaries by tag:", err);
    }
  }

  /**
   * 处理日记删除
   * @param {string} id - 要删除的日记ID
   */
  async function handleDeleteDiary(id) {
    try {
      await invoke("delete_diary", { id });
      
      // 从缓存中删除
      diaryCache.delete(id);
      
      // 从列表中删除
      diaries = diaries.filter(diary => diary.id !== id);
      
      // 如果删除的是当前选中的日记，清空选择
      if (selectedDiary && selectedDiary.id === id) {
        selectedDiary = null;
      }
      
      // 刷新图形视图
      if (graphCanvas) {
        await graphCanvas.refreshGraph();
      }
    } catch (err) {
      console.error("Error deleting diary:", err);
      error = "Failed to delete diary";
    }
  }
</script>

<main class="container">
  <header class="app-header">
    <h1>SecondBrian</h1>
    <p>Your secure, local diary with graph visualization</p>
  </header>

  <div class="app-container">
    <aside class="sidebar">
      <div class="sidebar-header">
        <div class="sidebar-title">
          {#if currentTag}
            <h2>Tagged: {currentTag}</h2>
            <button class="back-button" on:click={resetFilter}>
              <span>← All Entries</span>
            </button>
          {:else}
            <h2>Your Entries</h2>
          {/if}
        </div>
        <button class="new-button" on:click={createNewDiary}>New Entry</button>
      </div>
      
      {#if isLoading}
        <div class="loading">Loading entries...</div>
      {:else if error}
        <div class="error">{error}</div>
      {:else if diaries.length === 0}
        <div class="empty-state">
          <p>No entries yet. Create your first one!</p>
        </div>
      {:else}
        <ul class="diary-list">
          {#each diaries as diary}
            <li 
              class="diary-item" 
              class:active={selectedDiary && selectedDiary.id === diary.id}
            >
              <div class="diary-content" on:click={() => handleSelectDiary({ detail: { id: diary.id } })}>
                <div class="diary-title">{diary.title}</div>
                <div class="diary-date">{new Date(diary.created_at).toLocaleDateString()}</div>
                <div class="diary-tags">
                  {#each diary.tags as tag}
                    <span class="diary-tag">{tag}</span>
                  {/each}
                </div>
              </div>
              <button 
                class="delete-button"
                on:click={() => handleDeleteDiary(diary.id)}
                title="Delete diary"
              >
                ×
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </aside>
    
    <div class="main-content">
      <div class="tabs">
        <button 
          class="tab-button" 
          class:active={activeTab === "editor"}
          on:click={() => activeTab = "editor"}
        >
          Editor
        </button>
        <button 
          class="tab-button" 
          class:active={activeTab === "graph"}
          on:click={() => activeTab = "graph"}
        >
          Graph View
        </button>
      </div>
      
      <div class="tab-content">
        {#if activeTab === "editor"}
          <div class="editor-container">
            <Editor 
              entry={selectedDiary ?? { id: "", title: "", content: "", tags: [] }} 
              isNew={!selectedDiary}
              on:saved={handleDiarySaved}
            />
          </div>
        {:else if activeTab === "graph"}
          <div class="graph-container">
            <GraphCanvas 
              bind:this={graphCanvas}
              on:selectDiary={handleSelectDiary}
              on:selectTag={handleSelectTag}
            />
          </div>
        {/if}
      </div>
    </div>
  </div>
</main>

<style>
  .container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  
  .app-header {
    margin-bottom: 2rem;
    text-align: center;
  }
  
  .app-header h1 {
    margin: 0;
    font-size: 2.5rem;
    color: #333;
  }
  
  .app-header p {
    margin: 0.5rem 0 0;
    color: #666;
  }
  
  .app-container {
    display: flex;
    flex: 1;
    gap: 2rem;
    min-height: 0;
  }
  
  .sidebar {
    width: 300px;
    background-color: #f5f5f5;
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  
  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }
  
  .sidebar-header h2 {
    margin: 0;
    font-size: 1.2rem;
  }
  
  .new-button {
    background-color: #4CAF50;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    cursor: pointer;
    font-weight: bold;
  }
  
  .diary-list {
    list-style: none;
    padding: 0;
    margin: 0;
    overflow-y: auto;
    flex: 1;
  }
  
  .diary-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid #ddd;
    cursor: pointer;
    transition: background-color 0.2s;
  }
  
  .diary-item:hover {
    background-color: #eee;
  }
  
  .diary-item.active {
    background-color: #e3f2fd;
  }
  
  .diary-content {
    flex: 1;
  }
  
  .diary-title {
    font-weight: bold;
    margin-bottom: 0.5rem;
  }
  
  .diary-date {
    font-size: 0.8rem;
    color: #666;
    margin-bottom: 0.5rem;
  }
  
  .diary-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  
  .diary-tag {
    font-size: 0.7rem;
    background-color: #e0e0e0;
    padding: 0.1rem 0.5rem;
    border-radius: 10px;
  }
  
  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  
  .tabs {
    display: flex;
    border-bottom: 1px solid #ddd;
    margin-bottom: 1rem;
  }
  
  .tab-button {
    padding: 0.75rem 1.5rem;
    background: none;
    border: none;
    cursor: pointer;
    font-weight: 500;
    color: #666;
    border-bottom: 2px solid transparent;
  }
  
  .tab-button.active {
    color: #1976D2;
    border-bottom-color: #1976D2;
  }
  
  .tab-content {
    flex: 1;
    overflow: hidden;
  }
  
  .editor-container, .graph-container {
    height: 100%;
  }
  
  .loading, .error, .empty-state {
    padding: 2rem;
    text-align: center;
    color: #666;
  }
  
  .error {
    color: #d32f2f;
  }
  
  .sidebar-title {
    margin-bottom: 0.5rem;
  }

  .back-button {
    background: none;
    border: none;
    color: #1976D2;
    cursor: pointer;
    padding: 0;
    font-size: 0.8rem;
    display: inline-flex;
    align-items: center;
    margin-top: 0.25rem;
  }
  
  .back-button:hover {
    text-decoration: underline;
  }

  .delete-button {
    background: none;
    border: none;
    color: #d32f2f;
    font-size: 1.2rem;
    cursor: pointer;
    padding: 0.5rem;
    opacity: 0;
    transition: opacity 0.2s;
  }
  
  .diary-item:hover .delete-button {
    opacity: 1;
  }
  
  .delete-button:hover {
    color: #b71c1c;
  }
</style>
