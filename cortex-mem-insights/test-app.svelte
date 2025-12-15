<script>
  let count = 0;
  
  function increment() {
    count += 1;
  }
</script>

<main>
  <h1>测试应用</h1>
  <p>计数: {count}</p>
  <button on:click={increment}>增加</button>
</main>

<style>
  main {
    padding: 2rem;
    text-align: center;
  }
  
  button {
    padding: 0.5rem 1rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 0.25rem;
    cursor: pointer;
  }
</style>