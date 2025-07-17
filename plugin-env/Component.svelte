<script lang="ts">
    let content = '';
    function update(event: Event) {
        content = (event.target as HTMLInputElement).value;
    }
</script>

<div class="component">
    <input type="text" on:input={update} />
    <h1>{content}</h1>
</div>

<style>
    .component {
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: center;
        width: 100%;
        height: 100%;
    }
    .component > h1 {
        color: red;
    }
</style>
