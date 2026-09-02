<?php
namespace App\Controller;

use App\Service\AuthService;

class AuthController
{
    public function __construct(private AuthService $authService) {}

    public function login(): ?array
    {
        $email = $_POST['email'] ?? '';
        return $this->authService->login($email);
    }
}
