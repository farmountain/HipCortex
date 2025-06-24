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
    initMetrics();
});

let reqChart;
function initMetrics() {
    const ctx = document.getElementById('req-chart').getContext('2d');
    reqChart = new Chart(ctx, {
        type: 'line',
        data: { labels: [], datasets: [{ label: 'req/s', data: [] }] },
    });
    setInterval(updateMetrics, 2000);
}

async function updateMetrics() {
    const metrics = await window.__TAURI__.invoke('get_metrics');
    document.getElementById('cache-hits').innerText = 'cache hits: ' + metrics.cache_hits;
    const dataset = reqChart.data.datasets[0];
    dataset.data.push(metrics.request_rate);
    reqChart.data.labels.push('');
    if (dataset.data.length > 20) {
        dataset.data.shift();
        reqChart.data.labels.shift();
    }
    reqChart.update();
}
