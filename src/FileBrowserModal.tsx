import {
  Button,
  Modal,
  ModalBody,
  ModalCloseButton,
  ModalContent,
  ModalFooter,
  ModalHeader,
  ModalOverlay,
  Input,
  VStack,
  HStack,
  Text,
  Box,
  Divider,
  useToast,
  Spinner,
  Badge,
  IconButton,
  useDisclosure,
  AlertDialog,
  AlertDialogBody,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogContent,
  AlertDialogOverlay,
} from "@chakra-ui/react";
import { useState, useEffect, useRef } from "react";
import { VscFile, VscHistory, VscTrash } from "react-icons/vsc";

type FrozenDocument = {
  document_id: string;
  owner_token: string;
  language: string;
  file_extension: string;
  frozen_at: string;
  expires_at: string;
  file_size: number;
};

type FileBrowserModalProps = {
  isOpen: boolean;
  onClose: () => void;
  darkMode: boolean;
};

function FileBrowserModal({
  isOpen,
  onClose,
  darkMode,
}: FileBrowserModalProps) {
  const toast = useToast();
  const [ownerToken, setOwnerToken] = useState("");
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [documents, setDocuments] = useState<FrozenDocument[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [documentToDelete, setDocumentToDelete] = useState<string | null>(null);
  const { isOpen: isDeleteOpen, onOpen: onDeleteOpen, onClose: onDeleteClose } = useDisclosure();
  const cancelRef = useRef<HTMLButtonElement>(null);

  // Check for stored token on mount
  useEffect(() => {
    const storedToken = localStorage.getItem("rustpad_owner_token");
    if (storedToken) {
      setOwnerToken(storedToken);
      handleAuthenticate(storedToken);
    }
  }, []);

  async function handleAuthenticate(token?: string) {
    const tokenToUse = token || ownerToken;
    if (!tokenToUse) return;

    setIsLoading(true);
    try {
      const response = await fetch("/api/documents/list", {
        headers: {
          Authorization: `Bearer ${tokenToUse}`,
        },
      });

      if (!response.ok) {
        throw new Error("Invalid token or server error");
      }

      const docs: FrozenDocument[] = await response.json();
      setDocuments(docs);
      setIsAuthenticated(true);
      localStorage.setItem("rustpad_owner_token", tokenToUse);
      
      toast({
        title: "Authenticated",
        description: `Found ${docs.length} frozen document${docs.length !== 1 ? "s" : ""}`,
        status: "success",
        duration: 2000,
        isClosable: true,
      });
    } catch (error) {
      toast({
        title: "Authentication failed",
        description: "Invalid owner token",
        status: "error",
        duration: 3000,
        isClosable: true,
      });
      setIsAuthenticated(false);
      localStorage.removeItem("rustpad_owner_token");
    } finally {
      setIsLoading(false);
    }
  }

  function handleLogout() {
    setIsAuthenticated(false);
    setDocuments([]);
    setOwnerToken("");
    localStorage.removeItem("rustpad_owner_token");
  }

  function handleOpenDocument(docId: string) {
    window.location.hash = docId;
    onClose();
  }

  function formatDate(dateString: string) {
    const date = new Date(dateString);
    return date.toLocaleDateString() + " " + date.toLocaleTimeString([], { 
      hour: '2-digit', 
      minute: '2-digit' 
    });
  }

  function formatFileSize(bytes: number) {
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  }

  function getDaysUntilExpiry(expiresAt: string) {
    const now = new Date();
    const expiry = new Date(expiresAt);
    const diffTime = expiry.getTime() - now.getTime();
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));
    return diffDays;
  }

  function handleDeleteClick(docId: string, event: React.MouseEvent) {
    event.stopPropagation(); // Prevent opening the document
    setDocumentToDelete(docId);
    onDeleteOpen();
  }

  async function handleConfirmDelete() {
    if (!documentToDelete) return;

    try {
      const response = await fetch(`/api/documents/${documentToDelete}/delete`, {
        method: "DELETE",
        headers: {
          Authorization: `Bearer ${ownerToken}`,
        },
      });

      if (!response.ok) {
        throw new Error("Failed to delete document");
      }

      // Remove from local state
      setDocuments(documents.filter(d => d.document_id !== documentToDelete));

      toast({
        title: "Document deleted",
        description: "The frozen document has been removed",
        status: "success",
        duration: 3000,
        isClosable: true,
      });
    } catch (error) {
      toast({
        title: "Delete failed",
        description: error instanceof Error ? error.message : "Unknown error",
        status: "error",
        duration: 4000,
        isClosable: true,
      });
    } finally {
      onDeleteClose();
      setDocumentToDelete(null);
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="xl">
      <ModalOverlay />
      <ModalContent bgColor={darkMode ? "#1e1e1e" : "white"}>
        <ModalHeader color={darkMode ? "#cbcaca" : "inherit"}>
          My Frozen Files
        </ModalHeader>
        <ModalCloseButton color={darkMode ? "#cbcaca" : "inherit"} />
        <ModalBody>
          {!isAuthenticated ? (
            <VStack spacing={4}>
              <Text fontSize="sm" color={darkMode ? "#cbcaca" : "inherit"}>
                Enter your owner token to access your frozen documents. This
                token was generated when you first froze a document.
              </Text>
              <Input
                placeholder="Owner Token (UUID)"
                value={ownerToken}
                onChange={(e) => setOwnerToken(e.target.value)}
                bgColor={darkMode ? "#3c3c3c" : "white"}
                borderColor={darkMode ? "#3c3c3c" : "gray.200"}
                color={darkMode ? "#cbcaca" : "inherit"}
                onKeyPress={(e) => {
                  if (e.key === "Enter") handleAuthenticate();
                }}
              />
              <Button
                colorScheme="blue"
                onClick={() => handleAuthenticate()}
                isLoading={isLoading}
                isDisabled={!ownerToken}
                w="full"
              >
                Authenticate
              </Button>
            </VStack>
          ) : (
            <VStack spacing={3} align="stretch">
              <HStack justifyContent="space-between">
                <Text fontSize="sm" color={darkMode ? "#888" : "gray.600"}>
                  {documents.length} document{documents.length !== 1 ? "s" : ""}{" "}
                  found
                </Text>
                <Button size="xs" variant="ghost" onClick={handleLogout}>
                  Logout
                </Button>
              </HStack>

              <Divider />

              {isLoading ? (
                <Box textAlign="center" py={8}>
                  <Spinner color={darkMode ? "#cbcaca" : "inherit"} />
                </Box>
              ) : documents.length === 0 ? (
                <Text
                  textAlign="center"
                  py={8}
                  color={darkMode ? "#888" : "gray.500"}
                >
                  No frozen documents found
                </Text>
              ) : (
                documents.map((doc) => {
                  const daysLeft = getDaysUntilExpiry(doc.expires_at);
                  return (
                    <Box
                      key={doc.document_id}
                      p={3}
                      borderRadius="md"
                      borderWidth="1px"
                      borderColor={darkMode ? "#3c3c3c" : "gray.200"}
                      bgColor={darkMode ? "#252526" : "gray.50"}
                      cursor="pointer"
                      _hover={{
                        bgColor: darkMode ? "#2d2d30" : "gray.100",
                      }}
                      onClick={() => handleOpenDocument(doc.document_id)}
                    >
                      <HStack justifyContent="space-between" mb={2}>
                        <HStack flex={1}>
                          <VscFile />
                          <Text
                            fontWeight="semibold"
                            fontSize="sm"
                            color={darkMode ? "#cbcaca" : "inherit"}
                          >
                            {doc.document_id}.{doc.file_extension}
                          </Text>
                        </HStack>
                        <HStack spacing={2}>
                          <Badge
                            colorScheme={
                              daysLeft <= 3
                                ? "red"
                                : daysLeft <= 7
                                  ? "yellow"
                                  : "green"
                            }
                            fontSize="xs"
                          >
                            {daysLeft}d left
                          </Badge>
                          <IconButton
                            aria-label="Delete document"
                            icon={<VscTrash />}
                            size="xs"
                            colorScheme="red"
                            variant="ghost"
                            onClick={(e) => handleDeleteClick(doc.document_id, e)}
                          />
                        </HStack>
                      </HStack>
                      <HStack
                        spacing={4}
                        fontSize="xs"
                        color={darkMode ? "#888" : "gray.600"}
                      >
                        <Text>{doc.language}</Text>
                        <Text>•</Text>
                        <Text>{formatFileSize(doc.file_size)}</Text>
                        <Text>•</Text>
                        <HStack spacing={1}>
                          <VscHistory />
                          <Text>{formatDate(doc.frozen_at)}</Text>
                        </HStack>
                      </HStack>
                    </Box>
                  );
                })
              )}
            </VStack>
          )}
        </ModalBody>

        <ModalFooter>
          <Button variant="ghost" onClick={onClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>

      <AlertDialog
        isOpen={isDeleteOpen}
        leastDestructiveRef={cancelRef}
        onClose={onDeleteClose}
      >
        <AlertDialogOverlay>
          <AlertDialogContent bgColor={darkMode ? "#1e1e1e" : "white"}>
            <AlertDialogHeader
              fontSize="lg"
              fontWeight="bold"
              color={darkMode ? "#cbcaca" : "inherit"}
            >
              Delete Document
            </AlertDialogHeader>

            <AlertDialogBody color={darkMode ? "#cbcaca" : "inherit"}>
              Are you sure you want to delete this frozen document? This action
              cannot be undone.
            </AlertDialogBody>

            <AlertDialogFooter>
              <Button ref={cancelRef} onClick={onDeleteClose}>
                Cancel
              </Button>
              <Button colorScheme="red" onClick={handleConfirmDelete} ml={3}>
                Delete
              </Button>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialogOverlay>
      </AlertDialog>
    </Modal>
  );
}

export default FileBrowserModal;
