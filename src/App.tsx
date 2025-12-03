import { Box, Flex, HStack, Icon, Text, useToast } from "@chakra-ui/react";
import Editor from "@monaco-editor/react";
import { editor } from "monaco-editor/esm/vs/editor/editor.api";
import { useEffect, useRef, useState } from "react";
import { VscChevronRight, VscFolderOpened, VscGist } from "react-icons/vsc";
import useLocalStorageState from "use-local-storage-state";

import AiPanel from "./AiPanel";
import FileBrowserModal from "./FileBrowserModal";
import Footer from "./Footer";
import LoginModal from "./LoginModal";
import Sidebar from "./Sidebar";
import animals from "./animals.json";
import languages from "./languages.json";
import Rustpad, { UserInfo } from "./rustpad";
import useHash from "./useHash";

function getWsUri(id: string) {
  let url = new URL(`api/socket/${id}`, window.location.href);
  url.protocol = url.protocol == "https:" ? "wss:" : "ws:";
  return url.href;
}

function generateName() {
  return "Anonymous " + animals[Math.floor(Math.random() * animals.length)];
}

function generateHue() {
  return Math.floor(Math.random() * 360);
}

function App() {
  const toast = useToast();
  const [language, setLanguage] = useState("plaintext");
  const [connection, setConnection] = useState<
    "connected" | "disconnected" | "desynchronized"
  >("disconnected");
  const [users, setUsers] = useState<Record<number, UserInfo>>({});
  const [name, setName] = useLocalStorageState("name", {
    defaultValue: generateName,
  });
  const [hue, setHue] = useLocalStorageState("hue", {
    defaultValue: generateHue,
  });
  const [editor, setEditor] = useState<editor.IStandaloneCodeEditor>();
  const [darkMode, setDarkMode] = useLocalStorageState("darkMode", {
    defaultValue: false,
  });
  const rustpad = useRef<Rustpad>();
  const id = useHash();

  const [fileBrowserOpen, setFileBrowserOpen] = useState(false);
  const [loginModalOpen, setLoginModalOpen] = useState(false);
  const [aiPanelOpen, setAiPanelOpen] = useState(false);
  const [username, setUsername] = useLocalStorageState<string | null>("rustpad_username", {
    defaultValue: null,
  });
  const [password, setPassword] = useLocalStorageState<string | null>("rustpad_password", {
    defaultValue: null,
  });
  const [aiEnabled, setAiEnabled] = useLocalStorageState<boolean>("rustpad_ai_enabled", {
    defaultValue: false,
  });

  useEffect(() => {
    if (editor?.getModel()) {
      const model = editor.getModel()!;
      model.setValue("");
      model.setEOL(0); // LF
      rustpad.current = new Rustpad({
        uri: getWsUri(id),
        editor,
        onConnected: () => setConnection("connected"),
        onDisconnected: () => setConnection("disconnected"),
        onDesynchronized: () => {
          setConnection("desynchronized");
          toast({
            title: "Desynchronized with server",
            description: "Please save your work and refresh the page.",
            status: "error",
            duration: null,
          });
        },
        onChangeLanguage: (language) => {
          if (languages.includes(language)) {
            setLanguage(language);
          }
        },
        onChangeUsers: setUsers,
      });
      return () => {
        rustpad.current?.dispose();
        rustpad.current = undefined;
      };
    }
  }, [id, editor, toast, setUsers]);

  useEffect(() => {
    if (connection === "connected") {
      rustpad.current?.setInfo({ name, hue });
    }
  }, [connection, name, hue]);

  function handleLanguageChange(language: string) {
    setLanguage(language);
    if (rustpad.current?.setLanguage(language)) {
      toast({
        title: "Language updated",
        description: (
          <>
            All users are now editing in{" "}
            <Text as="span" fontWeight="semibold">
              {language}
            </Text>
            .
          </>
        ),
        status: "info",
        duration: 2000,
        isClosable: true,
      });
    }
  }

  function handleDarkModeChange() {
    setDarkMode(!darkMode);
  }

  async function handleFreeze() {
    // Check if user is logged in
    if (!username || !password) {
      setLoginModalOpen(true);
      return;
    }

    try {
      const authHeader = btoa(`${username}:${password}`);
      const response = await fetch(`/api/documents/${id}/freeze`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Authorization": `Basic ${authHeader}`,
        },
        body: JSON.stringify({
          language: language !== "plaintext" ? language : undefined,
        }),
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || "Failed to freeze document");
      }

      const result = await response.json();
      const expiryDate = new Date(result.expires_at);
      
      toast({
        title: "Document frozen!",
        description: (
          <>
            Saved as <Text as="span" fontWeight="semibold">{result.document_id}.{result.file_extension}</Text>.
            <br />
            Expires: {expiryDate.toLocaleDateString()}
          </>
        ),
        status: "success",
        duration: 6000,
        isClosable: true,
      });
    } catch (error) {
      toast({
        title: "Freeze failed",
        description: error instanceof Error ? error.message : "Unknown error",
        status: "error",
        duration: 4000,
        isClosable: true,
      });
    }
  }

  function handleDownload() {
    window.location.href = `/api/documents/${id}/download`;
    toast({
      title: "Downloading",
      description: "Your document is being downloaded",
      status: "info",
      duration: 2000,
      isClosable: true,
    });
  }

  function handleNewDocument() {
    // Generate a random document ID (similar to how the app generates initial IDs)
    const randomId = Math.random().toString(36).substring(2, 15);
    window.location.hash = randomId;
    toast({
      title: "New document created",
      description: "You can start editing or share the link with others",
      status: "success",
      duration: 3000,
      isClosable: true,
    });
  }

  function handleLoginSuccess(user: string, pass: string) {
    setUsername(user);
    setPassword(pass);
    // Update ai_enabled from localStorage (set by LoginModal)
    const aiEnabledStr = localStorage.getItem("rustpad_ai_enabled");
    setAiEnabled(aiEnabledStr === "true");
  }

  function handleFilesClick() {
    if (!username || !password) {
      setLoginModalOpen(true);
    } else {
      setFileBrowserOpen(true);
    }
  }

  function handleAskAI() {
    if (!username || !password) {
      setLoginModalOpen(true);
      return;
    }
    if (!aiEnabled) {
      toast({
        title: "AI features not enabled",
        description: "Contact an administrator to enable AI features for your account",
        status: "warning",
        duration: 4000,
        isClosable: true,
      });
      return;
    }
    setAiPanelOpen(true);
  }

  function handleApplyEdit(content: string) {
    if (editor) {
      const model = editor.getModel();
      if (model) {
        model.setValue(content);
      }
    }
  }

  return (
    <Flex
      direction="column"
      h="100vh"
      overflow="hidden"
      bgColor={darkMode ? "#1e1e1e" : "white"}
      color={darkMode ? "#cbcaca" : "inherit"}
    >
      <Box
        flexShrink={0}
        bgColor={darkMode ? "#333333" : "#e8e8e8"}
        color={darkMode ? "#cccccc" : "#383838"}
        textAlign="center"
        fontSize="sm"
        py={0.5}
      >
        Rustpad
      </Box>
      <Flex flex="1 0" minH={0}>
        <Sidebar
          documentId={id}
          connection={connection}
          darkMode={darkMode}
          language={language}
          currentUser={{ name, hue }}
          users={users}
          onDarkModeChange={handleDarkModeChange}
          onLanguageChange={handleLanguageChange}
          onChangeName={(name) => name.length > 0 && setName(name)}
          onChangeColor={() => setHue(generateHue())}
          onFreeze={handleFreeze}
          onDownload={handleDownload}
          onNewDocument={handleNewDocument}
          onAskAI={handleAskAI}
          aiEnabled={aiEnabled}
        />

        <Flex flex={1} minW={0} h="100%" direction="column" overflow="hidden">
          <HStack
            h={6}
            spacing={1}
            color="#888888"
            fontWeight="medium"
            fontSize="13px"
            px={3.5}
            flexShrink={0}
          >
            <Icon as={VscFolderOpened} fontSize="md" color="blue.500" />
            <Text>documents</Text>
            <Icon as={VscChevronRight} fontSize="md" />
            <Icon as={VscGist} fontSize="md" color="purple.500" />
            <Text>{id}</Text>
          </HStack>
          <Box flex={1} minH={0}>
            <Editor
              theme={darkMode ? "vs-dark" : "vs"}
              language={language}
              options={{
                automaticLayout: true,
                fontSize: 13,
              }}
              onMount={(editor) => setEditor(editor)}
            />
          </Box>
        </Flex>
      </Flex>
      <Footer onOpenFiles={handleFilesClick} />
      <FileBrowserModal
        isOpen={fileBrowserOpen}
        onClose={() => setFileBrowserOpen(false)}
        darkMode={darkMode}
        username={username}
        password={password}
        onAuthRequired={() => setLoginModalOpen(true)}
      />
      <LoginModal
        isOpen={loginModalOpen}
        onClose={() => setLoginModalOpen(false)}
        onSuccess={handleLoginSuccess}
        darkMode={darkMode}
      />
      <AiPanel
        isOpen={aiPanelOpen}
        onClose={() => setAiPanelOpen(false)}
        darkMode={darkMode}
        username={username}
        password={password}
        documentContent={editor?.getValue() || ""}
        onApplyEdit={handleApplyEdit}
      />
    </Flex>
  );
}

export default App;
