// main.js
import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import { homeDir } from '@tauri-apps/api/path';

let encryptFilePath = null;
let decryptFilePath = null;

// Tab switching
document.querySelectorAll('.tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));

    tab.classList.add('active');
    document.getElementById(tab.dataset.tab).classList.add('active');
  });
});

// Encrypt file upload
document.getElementById('encryptFileUpload').addEventListener('click', async () => {
  try {
    const file = await open({
      multiple: false,
      directory: false,
    });

    if (file) {
      encryptFilePath = file;
      const fileName = file.split(/[\\/]/).pop();
      document.getElementById('encryptFileName').textContent = fileName;
      document.getElementById('encryptFileUpload').classList.add('has-file');
      updateEncryptButton();
    }
  } catch (error) {
    showStatus('encryptStatus', 'Error selecting file: ' + error, 'error');
  }
});

// Decrypt file upload
document.getElementById('decryptFileUpload').addEventListener('click', async () => {
  try {
    const file = await open({
      multiple: false,
      directory: false,
      filters: [{
        name: 'Encrypted Files',
        extensions: ['encrypted']
      }]
    });

    if (file) {
      decryptFilePath = file;
      const fileName = file.split(/[\\/]/).pop();
      document.getElementById('decryptFileName').textContent = fileName;
      document.getElementById('decryptFileUpload').classList.add('has-file');
      updateDecryptButton();
    }
  } catch (error) {
    showStatus('decryptStatus', 'Error selecting file: ' + error, 'error');
  }
});

// Update button states
function updateEncryptButton() {
  const password = document.getElementById('encryptPassword').value;
  document.getElementById('encryptBtn').disabled = !encryptFilePath || !password;
}

function updateDecryptButton() {
  const password = document.getElementById('decryptPassword').value;
  document.getElementById('decryptBtn').disabled = !decryptFilePath || !password;
}

document.getElementById('encryptPassword').addEventListener('input', updateEncryptButton);
document.getElementById('decryptPassword').addEventListener('input', updateDecryptButton);

// Encrypt button
document.getElementById('encryptBtn').addEventListener('click', async () => {
  const btn = document.getElementById('encryptBtn');
  const password = document.getElementById('encryptPassword').value;
  const algorithm = document.getElementById('encryptAlgorithm').value;

  try {
    btn.disabled = true;
    btn.innerHTML = '<span class="loading"></span> Encrypting...';

    const outputDir = await save({
      defaultPath: encryptFilePath.split(/[\\/]/).pop() + '.encrypted',
    });

    if (!outputDir) {
      btn.disabled = false;
      btn.textContent = 'Encrypt File';
      return;
    }

    const outputDirPath = outputDir.split(/[\\/]/).slice(0, -1).join('/');

    const result = await invoke('encrypt_file', {
      filePath: encryptFilePath,
      password,
      algorithm,
      outputDir: outputDirPath
    });

    showStatus('encryptStatus', `File encrypted successfully!\nSaved to: ${result}`, 'success');

    // Reset form
    encryptFilePath = null;
    document.getElementById('encryptFileName').textContent = '';
    document.getElementById('encryptFileUpload').classList.remove('has-file');
    document.getElementById('encryptPassword').value = '';

  } catch (error) {
    showStatus('encryptStatus', 'Encryption failed: ' + error, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Encrypt File';
    updateEncryptButton();
  }
});

// Decrypt button
document.getElementById('decryptBtn').addEventListener('click', async () => {
  const btn = document.getElementById('decryptBtn');
  const password = document.getElementById('decryptPassword').value;

  try {
    btn.disabled = true;
    btn.innerHTML = '<span class="loading"></span> Decrypting...';

    const outputDir = await save({
      defaultPath: 'decrypted_file',
    });

    if (!outputDir) {
      btn.disabled = false;
      btn.textContent = 'Decrypt File';
      return;
    }

    const outputDirPath = outputDir.split(/[\\/]/).slice(0, -1).join('/');

    const result = await invoke('decrypt_file', {
      filePath: decryptFilePath,
      password,
      outputDir: outputDirPath
    });

    showStatus('decryptStatus', `File decrypted successfully!\nSaved to: ${result}`, 'success');

    // Reset form
    decryptFilePath = null;
    document.getElementById('decryptFileName').textContent = '';
    document.getElementById('decryptFileUpload').classList.remove('has-file');
    document.getElementById('decryptPassword').value = '';

  } catch (error) {
    showStatus('decryptStatus', 'Decryption failed: ' + error, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Decrypt File';
    updateDecryptButton();
  }
});

function showStatus(elementId, message, type) {
  const statusEl = document.getElementById(elementId);
  statusEl.textContent = message;
  statusEl.className = `status ${type}`;
  statusEl.style.display = 'block';

  setTimeout(() => {
    statusEl.style.display = 'none';
  }, 5000);
}
