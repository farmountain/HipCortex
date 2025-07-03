import React, { useEffect, useState } from 'react';
import axios from 'axios';

export default function DeployPanel() {
  const [buildLogs, setBuildLogs] = useState('');
  const [status, setStatus] = useState('idle');

  async function triggerDeploy() {
    try {
      await axios.post('/api/deploy');
    } catch (err) {
      console.error(err);
    }
  }

  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const resp = await axios.get('/api/deploy/status');
        setStatus(resp.data.status);
        setBuildLogs(resp.data.logs);
      } catch (err) {
        console.error(err);
      }
    }, 3000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div>
      <button onClick={triggerDeploy}>Deploy Now</button>
      <div>Status: {status}</div>
      <pre>{buildLogs}</pre>
    </div>
  );
}
