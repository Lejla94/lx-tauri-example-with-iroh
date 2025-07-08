const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// State management
let nodeInfo = null;
const connections = [];

// DOM Elements
const initBtn = document.getElementById('init-btn');
const nodeInfoDiv = document.getElementById('node-info');
const nodeIdSpan = document.getElementById('node-id');
const peerIdInput = document.getElementById('peer-id');
const messageInput = document.getElementById('message-content');
const sendBtn = document.getElementById('send-btn');
const connectionsContainer = document.getElementById('connections-container');
const connectionsList = document.getElementById('connections-list');
const receivedMessagesDiv = document.getElementById('received-messages');
const messagesList = document.getElementById('messages-list');

// Initialize P2P client
setupEventListeners();
initBtn.addEventListener('click', async () => {
  try {
    nodeInfo = await invoke('initialize_client');
    nodeIdSpan.textContent = nodeInfo.node_id;
    nodeInfoDiv.style.display = 'block';
    console.log('Client initialized:', nodeInfo);
  } catch (err) {
    console.error('Initialization failed:', err);
    alert(`Initialization failed: ${err}`);
  }
});

// Send message to peer (validation removed + form refresh added)
sendBtn.addEventListener('click', async () => {
  // Get values before clearing
  const peerId = peerIdInput.value;
  const content = messageInput.value;

  // Clear form immediately on submit
  peerIdInput.value = '';
  messageInput.value = '';

  try {
    await invoke('send_message', { peerId, content });
    console.log('Message sent successfully');
  } catch (err) {
    console.error('Message sending failed:', err);
    alert(`Message sending failed: ${err}`);
  }
});

// Set up Tauri event listeners
function setupEventListeners() {
  // Listen for incoming messages
  listen('message', (event) => {
    const payload = event.payload;
    displayMessage({
      sender: payload.sender,
      content: payload.content
    });
  }).catch(err => console.error('Failed to listen for messages:', err));

  // Listen for connection status updates
  listen('connection', (event) => {
    const { peer_id, status } = event.payload;
    updateConnectionStatus(peer_id, status);
  }).catch(err => console.error('Failed to listen for connections:', err));

  // Listen for errors
  listen('error', (event) => {
    console.error('P2P Error:', event.payload);
    alert(`P2P Error: ${event.payload}`);
  }).catch(err => console.error('Failed to listen for errors:', err));
}

// Update connection status
function updateConnectionStatus(peerId, status) {
  // Remove if disconnected
  if (status === 'disconnected') {
    const index = connections.findIndex(conn => conn.peer_id === peerId);
    if (index !== -1) connections.splice(index, 1);
  } 
  // Update or add connection
  else {
    const existing = connections.find(conn => conn.peer_id === peerId);
    if (existing) {
      existing.status = status;
    } else {
      connections.push({ peer_id: peerId, status });
    }
  }
  
  renderConnections();
}

// Render connections to UI
function renderConnections() {
  connectionsList.innerHTML = '';
  
  if (connections.length === 0) {
    connectionsContainer.style.display = 'none';
    return;
  }
  
  connectionsContainer.style.display = 'block';
  connections.forEach(conn => {
    const div = document.createElement('div');
    div.textContent = `${conn.peer_id} - ${conn.status}`;
    connectionsList.appendChild(div);
  });
}

// Display received message
function displayMessage(message) {
  receivedMessagesDiv.style.display = 'block';
  const li = document.createElement('li');
  li.textContent = `${message.sender}: ${message.content}`;
  messagesList.appendChild(li);
}
