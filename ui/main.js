async function loadGraph() {
    const data = await window.__TAURI__.invoke('get_symbolic_graph');
    document.getElementById('graph-data').innerText = JSON.stringify(data, null, 2);
}

async function runReflexion() {
    const loops = await window.__TAURI__.invoke('run_reflexion');
    document.getElementById('fsm-state').innerText = 'Loops run: ' + loops;
}

async function sendPerception() {
    const text = document.getElementById('perception-input').value;
    const log = await window.__TAURI__.invoke('send_perception', { text });
    document.getElementById('perception-log').innerText += log + '\n';
    document.getElementById('perception-input').value = '';
}

async function runCli() {
    const cmd = document.getElementById('cli-cmd').value;
    const out = await window.__TAURI__.invoke('cli_command', { cmd });
    document.getElementById('cli-out').innerText += out + '\n';
    document.getElementById('cli-cmd').value = '';
}

document.addEventListener('DOMContentLoaded', () => {
    loadGraph();
});
