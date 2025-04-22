-- MySQL dump 10.13  Distrib 9.3.0, for Linux (aarch64)
--
-- Host: localhost    Database: mydb
-- ------------------------------------------------------
-- Server version	9.3.0

/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
/*!40101 SET @OLD_CHARACTER_SET_RESULTS=@@CHARACTER_SET_RESULTS */;
/*!40101 SET @OLD_COLLATION_CONNECTION=@@COLLATION_CONNECTION */;
/*!50503 SET NAMES utf8mb4 */;
/*!40103 SET @OLD_TIME_ZONE=@@TIME_ZONE */;
/*!40103 SET TIME_ZONE='+00:00' */;
/*!40014 SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0 */;
/*!40014 SET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0 */;
/*!40101 SET @OLD_SQL_MODE=@@SQL_MODE, SQL_MODE='NO_AUTO_VALUE_ON_ZERO' */;
/*!40111 SET @OLD_SQL_NOTES=@@SQL_NOTES, SQL_NOTES=0 */;

--
-- Current Database: `mydb`
--

CREATE DATABASE /*!32312 IF NOT EXISTS*/ `mydb` /*!40100 DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci */ /*!80016 DEFAULT ENCRYPTION='N' */;

USE `mydb`;

--
-- Table structure for table `products`
--

DROP TABLE IF EXISTS `products`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `products` (
  `id` int NOT NULL AUTO_INCREMENT,
  `name` varchar(100) DEFAULT NULL,
  `tags` json DEFAULT NULL,
  `price` decimal(10,2) DEFAULT NULL,
  `meta` json DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=4 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `products`
--

LOCK TABLES `products` WRITE;
/*!40000 ALTER TABLE `products` DISABLE KEYS */;
INSERT INTO `products` VALUES (1,'products','[\"electronics\", \"computer\", \"16GB RAM\"]',1299.99,'{\"brand\": \"BrandX\", \"features\": {\"color\": \"silver\", \"weight\": 2.5}}'),(2,'Phone','[\"electronics\", \"mobile\", \"Android\"]',799.50,'{\"brand\": \"BrandY\", \"features\": {\"color\": \"black\", \"weight\": 0.2}}'),(3,'Desk','[\"furniture\", \"wood\", \"large\"]',250.00,'{\"brand\": \"BrandZ\", \"features\": {\"material\": \"oak\", \"adjustable\": true}}');
/*!40000 ALTER TABLE `products` ENABLE KEYS */;
 
LOCK TABLES `cars` WRITE;
/*!40000 ALTER TABLE `products` DISABLE KEYS */;
INSERT INTO `cars` VALUES (1,'cars','[\"electronics\", \"computer\", \"16GB RAM\"]',1299.99,'{\"brand\": \"BrandX\", \"features\": {\"color\": \"silver\", \"weight\": 2.5}}'),(2,'Phone','[\"electronics\", \"mobile\", \"Android\"]',799.50,'{\"brand\": \"BrandY\", \"features\": {\"color\": \"black\", \"weight\": 0.2}}'),(3,'Desk','[\"furniture\", \"wood\", \"large\"]',250.00,'{\"brand\": \"BrandZ\", \"features\": {\"material\": \"oak\", \"adjustable\": true}}');
/*!40000 ALTER TABLE `products` ENABLE KEYS */;
UNLOCK TABLES;
LOCK TABLES `home` WRITE;
/*!40000 ALTER TABLE `products` DISABLE KEYS */;
INSERT INTO `home` VALUES (1,'home','[\"electronics\", \"computer\", \"16GB RAM\"]',1299.99,'{\"brand\": \"BrandX\", \"features\": {\"color\": \"silver\", \"weight\": 2.5}}'),(2,'Phone','[\"electronics\", \"mobile\", \"Android\"]',799.50,'{\"brand\": \"BrandY\", \"features\": {\"color\": \"black\", \"weight\": 0.2}}'),(3,'Desk','[\"furniture\", \"wood\", \"large\"]',250.00,'{\"brand\": \"BrandZ\", \"features\": {\"material\": \"oak\", \"adjustable\": true}}');
/*!40000 ALTER TABLE `products` ENABLE KEYS */;
UNLOCK TABLES;
/*!40103 SET TIME_ZONE=@OLD_TIME_ZONE */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;

-- Dump completed on 2025-04-19  1:45:18
