import {
  Box,
  Button,
  Modal,
  ModalBody,
  ModalCloseButton,
  ModalContent,
  ModalHeader,
  ModalOverlay,
  VStack,
  HStack,
  Text,
  Table,
  Thead,
  Tbody,
  Tr,
  Th,
  Td,
  Badge,
  IconButton,
  useToast,
  Spinner,
  Switch,
  AlertDialog,
  AlertDialogBody,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogContent,
  AlertDialogOverlay,
  useDisclosure,
} from "@chakra-ui/react";
import { useState, useEffect, useRef } from "react";
import { VscTrash, VscRefresh } from "react-icons/vsc";

type User = {
  username: string;
  created_at: string;
  ai_enabled: boolean;
  is_admin: boolean;
};

type AdminPanelProps = {
  isOpen: boolean;
  onClose: () => void;
  darkMode: boolean;
  username: string | null;
  password: string | null;
};

function AdminPanel({
  isOpen,
  onClose,
  darkMode,
  username,
  password,
}: AdminPanelProps) {
  const toast = useToast();
  const [users, setUsers] = useState<User[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [deleteUser, setDeleteUser] = useState<string | null>(null);
  const { isOpen: isDeleteOpen, onOpen: onDeleteOpen, onClose: onDeleteClose } = useDisclosure();
  const cancelRef = useRef<HTMLButtonElement>(null);

  // Load users when panel opens
  useEffect(() => {
    if (isOpen) {
      loadUsers();
    }
  }, [isOpen]);

  async function loadUsers() {
    if (!username || !password) return;

    setIsLoading(true);
    try {
      const authHeader = btoa(`${username}:${password}`);
      const response = await fetch("/api/admin/users", {
        headers: {
          Authorization: `Basic ${authHeader}`,
        },
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || "Failed to load users");
      }

      const data = await response.json();
      setUsers(data);
    } catch (error) {
      toast({
        title: "Failed to load users",
        description: error instanceof Error ? error.message : "Unknown error",
        status: "error",
        duration: 4000,
        isClosable: true,
      });
    } finally {
      setIsLoading(false);
    }
  }

  async function toggleAiAccess(targetUsername: string, currentStatus: boolean) {
    if (!username || !password) return;

    try {
      const authHeader = btoa(`${username}:${password}`);
      const response = await fetch(`/api/admin/users/${targetUsername}/ai`, {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Basic ${authHeader}`,
        },
        body: JSON.stringify({ ai_enabled: !currentStatus }),
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || "Failed to update AI access");
      }

      // Update local state
      setUsers(users.map(u => 
        u.username === targetUsername 
          ? { ...u, ai_enabled: !currentStatus }
          : u
      ));

      toast({
        title: "AI access updated",
        description: `AI access ${!currentStatus ? "enabled" : "disabled"} for ${targetUsername}`,
        status: "success",
        duration: 3000,
        isClosable: true,
      });
    } catch (error) {
      toast({
        title: "Failed to update AI access",
        description: error instanceof Error ? error.message : "Unknown error",
        status: "error",
        duration: 4000,
        isClosable: true,
      });
    }
  }

  function confirmDeleteUser(targetUsername: string) {
    setDeleteUser(targetUsername);
    onDeleteOpen();
  }

  async function handleDeleteUser() {
    if (!username || !password || !deleteUser) return;

    // Prevent deleting yourself
    if (deleteUser === username) {
      toast({
        title: "Cannot delete yourself",
        description: "You cannot delete your own admin account",
        status: "warning",
        duration: 3000,
        isClosable: true,
      });
      onDeleteClose();
      return;
    }

    try {
      const authHeader = btoa(`${username}:${password}`);
      const response = await fetch(`/api/admin/users/${deleteUser}`, {
        method: "DELETE",
        headers: {
          Authorization: `Basic ${authHeader}`,
        },
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || "Failed to delete user");
      }

      // Remove from local state
      setUsers(users.filter(u => u.username !== deleteUser));

      toast({
        title: "User deleted",
        description: `${deleteUser} has been removed`,
        status: "success",
        duration: 3000,
        isClosable: true,
      });
    } catch (error) {
      toast({
        title: "Failed to delete user",
        description: error instanceof Error ? error.message : "Unknown error",
        status: "error",
        duration: 4000,
        isClosable: true,
      });
    } finally {
      onDeleteClose();
      setDeleteUser(null);
    }
  }

  const totalUsers = users.length;
  const aiEnabledUsers = users.filter(u => u.ai_enabled).length;
  const adminUsers = users.filter(u => u.is_admin).length;

  return (
    <>
      <Modal isOpen={isOpen} onClose={onClose} size="4xl">
        <ModalOverlay />
        <ModalContent bgColor={darkMode ? "#1e1e1e" : "white"}>
          <ModalHeader
            borderBottomWidth="1px"
            borderColor={darkMode ? "#3c3c3c" : "gray.200"}
            color={darkMode ? "#cbcaca" : "inherit"}
          >
            <HStack justify="space-between">
              <Text>Admin Panel</Text>
              <IconButton
                icon={isLoading ? <Spinner size="sm" /> : <VscRefresh />}
                aria-label="Refresh users"
                size="sm"
                onClick={loadUsers}
                isDisabled={isLoading}
                variant="ghost"
                color={darkMode ? "#cbcaca" : "inherit"}
              />
            </HStack>
          </ModalHeader>
          <ModalCloseButton color={darkMode ? "#cbcaca" : "inherit"} />
          
          <ModalBody p={4}>
            {/* Statistics */}
            <HStack spacing={4} mb={6} wrap="wrap">
              <Box
                p={4}
                borderRadius="md"
                bgColor={darkMode ? "#2d3748" : "blue.50"}
                flex={1}
                minW="150px"
              >
                <Text fontSize="2xl" fontWeight="bold" color={darkMode ? "#cbcaca" : "inherit"}>
                  {totalUsers}
                </Text>
                <Text fontSize="sm" color={darkMode ? "#888" : "gray.600"}>
                  Total Users
                </Text>
              </Box>
              
              <Box
                p={4}
                borderRadius="md"
                bgColor={darkMode ? "#2d3748" : "green.50"}
                flex={1}
                minW="150px"
              >
                <Text fontSize="2xl" fontWeight="bold" color={darkMode ? "#cbcaca" : "inherit"}>
                  {aiEnabledUsers}
                </Text>
                <Text fontSize="sm" color={darkMode ? "#888" : "gray.600"}>
                  AI Enabled
                </Text>
              </Box>
              
              <Box
                p={4}
                borderRadius="md"
                bgColor={darkMode ? "#2d3748" : "red.50"}
                flex={1}
                minW="150px"
              >
                <Text fontSize="2xl" fontWeight="bold" color={darkMode ? "#cbcaca" : "inherit"}>
                  {adminUsers}
                </Text>
                <Text fontSize="sm" color={darkMode ? "#888" : "gray.600"}>
                  Administrators
                </Text>
              </Box>
            </HStack>

            {/* Users Table */}
            {isLoading && users.length === 0 ? (
              <VStack py={8} color={darkMode ? "#888" : "gray.500"}>
                <Spinner />
                <Text>Loading users...</Text>
              </VStack>
            ) : (
              <Box overflowX="auto">
                <Table size="sm" variant="simple">
                  <Thead>
                    <Tr>
                      <Th color={darkMode ? "#888" : "inherit"}>Username</Th>
                      <Th color={darkMode ? "#888" : "inherit"}>Created</Th>
                      <Th color={darkMode ? "#888" : "inherit"}>Roles</Th>
                      <Th color={darkMode ? "#888" : "inherit"}>AI Access</Th>
                      <Th color={darkMode ? "#888" : "inherit"}>Actions</Th>
                    </Tr>
                  </Thead>
                  <Tbody>
                    {users.map((user) => (
                      <Tr key={user.username}>
                        <Td color={darkMode ? "#cbcaca" : "inherit"}>
                          <Text fontWeight={user.username === username ? "bold" : "normal"}>
                            {user.username}
                            {user.username === username && " (You)"}
                          </Text>
                        </Td>
                        <Td color={darkMode ? "#888" : "gray.600"} fontSize="xs">
                          {new Date(user.created_at).toLocaleDateString()}
                        </Td>
                        <Td>
                          <HStack spacing={1}>
                            {user.is_admin && (
                              <Badge colorScheme="red" fontSize="xs">
                                Admin
                              </Badge>
                            )}
                            {user.ai_enabled && (
                              <Badge colorScheme="purple" fontSize="xs">
                                AI
                              </Badge>
                            )}
                          </HStack>
                        </Td>
                        <Td>
                          <Switch
                            size="sm"
                            isChecked={user.ai_enabled}
                            onChange={() => toggleAiAccess(user.username, user.ai_enabled)}
                            colorScheme="green"
                          />
                        </Td>
                        <Td>
                          <IconButton
                            icon={<VscTrash />}
                            aria-label="Delete user"
                            size="sm"
                            colorScheme="red"
                            variant="ghost"
                            onClick={() => confirmDeleteUser(user.username)}
                            isDisabled={user.username === username}
                          />
                        </Td>
                      </Tr>
                    ))}
                  </Tbody>
                </Table>
              </Box>
            )}
          </ModalBody>
        </ModalContent>
      </Modal>

      {/* Delete Confirmation Dialog */}
      <AlertDialog
        isOpen={isDeleteOpen}
        leastDestructiveRef={cancelRef}
        onClose={onDeleteClose}
      >
        <AlertDialogOverlay>
          <AlertDialogContent bgColor={darkMode ? "#1e1e1e" : "white"}>
            <AlertDialogHeader color={darkMode ? "#cbcaca" : "inherit"}>
              Delete User
            </AlertDialogHeader>

            <AlertDialogBody color={darkMode ? "#cbcaca" : "inherit"}>
              Are you sure you want to delete <strong>{deleteUser}</strong>?
              This will permanently remove their account and all associated data.
            </AlertDialogBody>

            <AlertDialogFooter>
              <Button ref={cancelRef} onClick={onDeleteClose}>
                Cancel
              </Button>
              <Button colorScheme="red" onClick={handleDeleteUser} ml={3}>
                Delete
              </Button>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialogOverlay>
      </AlertDialog>
    </>
  );
}

export default AdminPanel;
