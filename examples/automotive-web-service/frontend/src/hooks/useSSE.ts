import { useState, useCallback, useRef } from 'react';
import type { Message } from '../types';

export function useSSE(backendUrl: string) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [currentStreamingMessage, setCurrentStreamingMessage] = useState<string>('');
  const eventSourceRef = useRef<EventSource | null>(null);

  const sendMessage = useCallback(async (content: string) => {
    console.log('sendMessage called with:', content);

    // Close any existing connection
    if (eventSourceRef.current) {
      console.log('Closing existing EventSource connection');
      eventSourceRef.current.close();
      eventSourceRef.current = null;
    }

    // Add user message
    const userMessage: Message = {
      role: 'user',
      content,
      timestamp: new Date(),
    };

    console.log('Adding user message, current messages count:', messages.length);
    setMessages(prev => {
      const updated = [...prev, userMessage];
      console.log('User message added, new count:', updated.length);
      return updated;
    });

    setIsStreaming(true);
    setCurrentStreamingMessage('');

    try {
      // Build query parameters
      const params = new URLSearchParams({
        message: content,
        ...(sessionId && { session_id: sessionId }),
      });

      const url = `${backendUrl}/chat/stream?${params}`;
      console.log('Creating EventSource with URL:', url);
      console.log('Session ID being sent:', sessionId);

      const eventSource = new EventSource(url);
      eventSourceRef.current = eventSource;
      console.log('EventSource created, readyState:', eventSource.readyState);

      let accumulatedText = '';

      eventSource.addEventListener('open', () => {
        console.log('EventSource connection opened successfully');
      });

      eventSource.addEventListener('session', (event) => {
        console.log('Session event received:', event.data);
        const data = JSON.parse(event.data);
        setSessionId(data.session_id);
      });

      eventSource.addEventListener('delta', (event) => {
        const data = JSON.parse(event.data);
        accumulatedText += data.text;
        console.log('Delta received:', data.text, '| Accumulated:', accumulatedText.substring(0, 50) + '...');
        setCurrentStreamingMessage(accumulatedText);
      });

      eventSource.addEventListener('done', (event) => {
        const data = JSON.parse(event.data);
        const finalText = data.text || accumulatedText;
        console.log('Done event received, finalText:', finalText);

        const finalMessage: Message = {
          role: 'assistant',
          content: finalText,
          timestamp: new Date(),
        };

        console.log('Adding final message to array:', finalMessage);
        setMessages(prev => {
          const updated = [...prev, finalMessage];
          console.log('Messages array updated, new length:', updated.length);
          return updated;
        });

        // Delay clearing streaming state to ensure message is added first
        setTimeout(() => {
          setCurrentStreamingMessage('');
          setIsStreaming(false);
        }, 50);

        eventSource.close();
        eventSourceRef.current = null;
      });

      eventSource.addEventListener('error', (event) => {
        console.error('SSE Error event (custom):', event);
        try {
          const messageEvent = event as MessageEvent;
          if (messageEvent.data) {
            const data = JSON.parse(messageEvent.data);
            const errorMessage: Message = {
              role: 'assistant',
              content: `Error: ${data.error}`,
              timestamp: new Date(),
            };
            setMessages(prev => [...prev, errorMessage]);
            setCurrentStreamingMessage('');
            setIsStreaming(false);
            eventSource.close();
            eventSourceRef.current = null;
          }
        } catch (e) {
          console.error('Failed to parse error event:', e);
        }
      });

      eventSource.onerror = (error) => {
        console.error('EventSource connection error (onerror):', error);
        console.log('EventSource readyState:', eventSource.readyState);

        // If connection closed normally (readyState 2), finalize with accumulated content
        if (eventSource.readyState === EventSource.CLOSED) {
          console.log('Connection closed, finalizing with accumulated text');
          const finalText = accumulatedText;

          if (finalText) {
            const finalMessage: Message = {
              role: 'assistant',
              content: finalText,
              timestamp: new Date(),
            };
            console.log('Adding final message from onerror:', finalMessage);
            setMessages(prev => [...prev, finalMessage]);
          }

          setCurrentStreamingMessage('');
          setIsStreaming(false);
          eventSourceRef.current = null;
        }
      };

    } catch (error) {
      console.error('Error sending message:', error);
      setIsStreaming(false);
    }
  }, [backendUrl, sessionId]);

  return {
    messages,
    sendMessage,
    isStreaming,
    sessionId,
    currentStreamingMessage,
  };
}